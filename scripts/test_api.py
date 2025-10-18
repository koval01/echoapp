"""
Pretty-printed logging version of the Telegram init-data generator + tester.
Everything is in English (no Russian).

Features:
- Structured logging using the standard `logging` module.
- Optional pretty console output using `rich` (falls back to plain logging).
- Clear, human-readable messages for HMAC / Ed25519 strings and computed values.
- Shortened JWT display for logs.
- Command-line switches for skipping the 60s wait and for verbose output.

Usage:
    export BOT_TOKEN=123456:ABC
    python telegram_init_data_generator_pretty_logs.py [--no-wait] [--verbose]

If you want colored output install `rich`:
    pip install rich

"""

from __future__ import annotations

import os
import hmac
import hashlib
import base64
import time
import json
import argparse
import sys
import logging
from urllib.parse import quote
from typing import Any, Dict, Optional

import requests

try:
    # `rich` is optional. If present we will use it for nicer console printing.
    from rich.console import Console
    from rich.logging import RichHandler
    from rich.panel import Panel
    from rich.syntax import Syntax
    RICH_AVAILABLE = True
except Exception:
    RICH_AVAILABLE = False

from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization


# -----------------------------
# Logging / Console utilities
# -----------------------------

def configure_logging(verbose: bool = False) -> logging.Logger:
    """Configure and return a module-level logger.

    If `rich` is installed, use RichHandler for nicer output. Otherwise use a
    standard formatter with timestamps.
    """
    level = logging.DEBUG if verbose else logging.INFO

    if RICH_AVAILABLE:
        console = Console()
        handler = RichHandler(console=console, rich_tracebacks=True)
        # RichHandler ignores formatter for most parts
        formatter = logging.Formatter("%(message)s")
    else:
        handler = logging.StreamHandler()
        formatter = logging.Formatter(
            "%(asctime)s %(levelname)-8s %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
        )
        handler.setFormatter(formatter)

    logger = logging.getLogger("telegram_init_test")
    logger.handlers = []
    logger.setLevel(level)
    logger.addHandler(handler)
    logger.propagate = False

    return logger


def pretty_panel(logger: logging.Logger, title: str, text: str) -> None:
    """If rich is available, print a panel. Otherwise log normally."""
    if RICH_AVAILABLE:
        Console().print(Panel(text, title=title, expand=False))
    else:
        logger.info("%s:\n%s", title, text)


# -----------------------------
# Main generator class
# -----------------------------

class TelegramInitDataGenerator:
    """Helper that creates init-data payload compatible with Telegram WebApps.

    It also supports generating and signing an ed25519 signature and computing
    the HMAC-SHA256 hash used by the server-side verification logic.
    """

    def __init__(self, bot_token: str, public_key_hex: Optional[str] = None, private_key_hex: Optional[str] = None):
        self.bot_token = bot_token
        self.bot_id = bot_token.split(":")[0]

        if private_key_hex:
            self.private_key = ed25519.Ed25519PrivateKey.from_private_bytes(
                bytes.fromhex(private_key_hex))
        else:
            self.private_key = ed25519.Ed25519PrivateKey.generate()

        if public_key_hex:
            self.public_key = ed25519.Ed25519PublicKey.from_public_bytes(
                bytes.fromhex(public_key_hex))
        else:
            self.public_key = self.private_key.public_key()

    def get_public_key_hex(self) -> str:
        return self.public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        ).hex()

    def generate_secret_key(self) -> bytes:
        # This is the secret used to compute HMAC for Telegram WebApp data.
        return hmac.new(b"WebAppData", self.bot_token.encode(), hashlib.sha256).digest()

    def calculate_hash(self, data_check_string: str) -> str:
        secret_key = self.generate_secret_key()
        return hmac.new(secret_key, data_check_string.encode(), hashlib.sha256).hexdigest()

    def sign_ed25519(self, data_check_string: str) -> str:
        signature = self.private_key.sign(data_check_string.encode())
        return base64.urlsafe_b64encode(signature).decode("utf-8").rstrip("=")

    def _pairs_from_params(self, params: Dict[str, Any]) -> list[tuple[str, str]]:
        pairs: list[tuple[str, str]] = []
        for key in sorted(params.keys()):
            value = params[key]
            if isinstance(value, (dict, list)):
                value = json.dumps(value, separators=(
                    ",", ":"), ensure_ascii=False)
            pairs.append((key, str(value)))
        return pairs

    def generate_init_data(self, user_data: Optional[Dict[str, Any]] = None) -> str:
        if user_data is None:
            user_data = {}

        auth_date = int(time.time())

        params = user_data.copy()
        params["auth_date"] = str(auth_date)

        # Build Ed25519 data-check-string (sorted by keys and joined with '\n')
        ed25519_pairs = self._pairs_from_params(params)
        ed25519_data_check_string = "\n".join(
            [f"{k}={v}" for k, v in ed25519_pairs])

        final_ed25519_data_string = f"{self.bot_id}:WebAppData\n{ed25519_data_check_string}"
        signature = self.sign_ed25519(final_ed25519_data_string)

        # For HMAC we include the signature in the params when computing the final hash
        hmac_params = params.copy()
        hmac_params["signature"] = signature
        hmac_pairs = self._pairs_from_params(hmac_params)
        hmac_data_check_string = "\n".join([f"{k}={v}" for k, v in hmac_pairs])

        hash_value = self.calculate_hash(hmac_data_check_string)

        # final URL-encoded style init_data string
        final_params = self._pairs_from_params(params)
        final_params.append(("signature", signature))
        final_params.append(("hash", hash_value))

        encoded_pairs = []
        for key, value in final_params:
            # `user` and `receiver` can contain JSON blobs, so quote them strictly
            if key in ["user", "receiver"]:
                encoded_value = quote(value, safe="")
            else:
                encoded_value = value
            encoded_pairs.append(f"{key}={encoded_value}")

        init_data = "&".join(encoded_pairs)
        return init_data

    def generate_realistic_user_data(self, user_id: int = 1234567890) -> Dict[str, Any]:
        return {
            "user": {
                "id": user_id,
                "first_name": "Test",
                "last_name": "User",
                "username": "test_user",
                "language_code": "en",
                "allows_write_to_pm": True,
                "photo_url": "https://t.me/i/userpic/320/nothing.svg",
            },
            "chat_instance": "-1234567890",
            "chat_type": "private",
        }


# -----------------------------
# Helper utilities for requests & pretty printing
# -----------------------------

def shorten_jwt(jwt: str, keep: int = 40) -> str:
    if not jwt:
        return ""
    if len(jwt) <= keep:
        return jwt
    return f"{jwt[:keep]}... (len={len(jwt)})"


def pretty_json(data: Any) -> str:
    try:
        return json.dumps(data, indent=2, ensure_ascii=False)
    except Exception:
        return str(data)


def perform_test_sequence(generator: TelegramInitDataGenerator, base_url: str, no_wait: bool, logger: logging.Logger) -> None:
    """Perform the same sequence as the original script but log prettily.

    1. Build init_data
    2. Print Ed25519 / HMAC data check strings and computed values
    3. Call /v1/auth/init with X-InitData header
    4. Read cookie __Host-auth_token and call /v1/user/me
    5. Optionally wait for 60 seconds and test token invalidation
    """

    user_data = generator.generate_realistic_user_data()
    init_data = generator.generate_init_data(user_data=user_data)

    params = generator.generate_realistic_user_data()
    params["auth_date"] = str(int(time.time()))

    ed25519_pairs = []
    for key in sorted(params.keys()):
        value = params[key]
        if isinstance(value, (dict, list)):
            value = json.dumps(value, separators=(
                ",", ":"), ensure_ascii=False)
        ed25519_pairs.append((key, value))
    ed25519_dcs = "\n".join([f"{k}={v}" for k, v in ed25519_pairs])

    pretty_panel(logger, "Ed25519 Data Check String", ed25519_dcs)

    signature = generator.sign_ed25519(
        f"{generator.bot_id}:WebAppData\n{ed25519_dcs}")

    hmac_params = params.copy()
    hmac_params["signature"] = signature
    hmac_pairs = []
    for key in sorted(hmac_params.keys()):
        value = hmac_params[key]
        if isinstance(value, (dict, list)):
            value = json.dumps(value, separators=(
                ",", ":"), ensure_ascii=False)
        hmac_pairs.append((key, value))
    hmac_dcs = "\n".join([f"{k}={v}" for k, v in hmac_pairs])

    pretty_panel(logger, "HMAC Data Check String", hmac_dcs)

    hash_value = generator.calculate_hash(hmac_dcs)
    pretty_panel(logger, "Computed Values",
                 f"Hash: {hash_value}\nSignature: {signature}")

    url_init = f"{base_url.rstrip('/')}/v1/auth/init"
    logger.info("Sending GET %s with X-InitData header (length=%d)",
                url_init, len(init_data))

    try:
        response = requests.get(
            url_init, headers={"X-InitData": init_data}, timeout=10)
    except Exception as exc:
        logger.exception("Request to %s failed: %s", url_init, exc)
        raise

    logger.info("/v1/auth/init -> status %d", response.status_code)

    # Print response headers (concise)
    headers_text = "\n".join(
        [f"{k}: {v}" for k, v in response.headers.items()])
    pretty_panel(logger, "Response Headers", headers_text)

    cookies = response.cookies.get_dict()
    if not cookies:
        logger.warning("No cookies received from auth init response.")
    else:
        logger.info("Cookies: %s", json.dumps(cookies, ensure_ascii=False))

    jwt_ = cookies.get("__Host-auth_token")
    if not jwt_:
        logger.error(
            "Auth cookie '__Host-auth_token' not found. Aborting test sequence.")
        return

    logger.info("Received JWT (short): %s", shorten_jwt(jwt_))

    # Call user/me
    url_me = f"{base_url.rstrip('/')}/v1/user/me"
    logger.info("Requesting user info from %s", url_me)
    try:
        response = requests.get(
            url_me, headers={"Authorization": f"Bearer {jwt_}"}, timeout=10)
    except Exception as exc:
        logger.exception("Request to %s failed: %s", url_me, exc)
        raise

    if response.status_code != 200:
        logger.error("/v1/user/me returned status %d", response.status_code)
        pretty_panel(logger, "User Endpoint Response", response.text)
        return

    try:
        data = response.json().get("data")
    except Exception:
        logger.exception("Failed to parse JSON from user/me response")
        pretty_panel(logger, "Raw Response", response.text)
        return

    pretty_panel(logger, "User Data", pretty_json(data))

    # Run a few assertions (non-fatal, logged)
    expected = {
        "telegram_id": 1234567890,
        "first_name": "Test",
        "last_name": "User",
        "username": "test_user",
        "is_admin": False,
    }

    mismatches = []
    for k, v in expected.items():
        actual_val = data.get(k)
        if actual_val != v:
            mismatches.append((k, v, actual_val))

    if mismatches:
        for k, expected_v, actual_v in mismatches:
            logger.warning("Mismatch for %s: expected=%r actual=%r",
                           k, expected_v, actual_v)
    else:
        logger.info("All expected user fields matched.")

    if no_wait:
        logger.info("Skipping token lifetime wait (--no-wait provided).")
        return

    logger.info("Waiting 60 seconds to test token expiration...")
    time.sleep(60)

    # Try to re-use the same init data after token expiry - server should reject
    try:
        response = requests.get(
            url_init, headers={"X-InitData": init_data}, timeout=10)
    except Exception as exc:
        logger.exception("Request to %s failed: %s", url_init, exc)
        raise

    logger.info("Second /v1/auth/init -> status %d", response.status_code)
    if response.status_code == 401:
        logger.info("Token correctly rejected after expiry period.")
    else:
        logger.warning(
            "Unexpected status after expiration wait: %d", response.status_code)


# -----------------------------
# CLI entry point
# -----------------------------

def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Telegram WebApp init-data generator & pretty tester")
    parser.add_argument("--base-url", default=os.getenv("BASE_URL",
                                                        "http://localhost:8000"), help="Base URL of the API (default: %(default)s)")
    parser.add_argument("--no-wait", action="store_true",
                        help="Do not wait 60 seconds to test token expiration")
    parser.add_argument("--verbose", action="store_true",
                        help="Verbose output (DEBUG level)")
    parser.add_argument("--private-key-hex", default=os.getenv("TEST_PRIVATE_KEY"),
                        help="Hex of Ed25519 private key (optional)")
    parser.add_argument("--public-key-hex", default=os.getenv("TEST_PUBLIC_KEY"),
                        help="Hex of Ed25519 public key (optional)")
    parser.add_argument("--bot-token", default=os.getenv("BOT_TOKEN"),
                        help="Bot token (or set BOT_TOKEN env var)")
    return parser.parse_args(argv)


def main(argv: Optional[list[str]] = None) -> None:
    args = parse_args(argv)

    logger = configure_logging(args.verbose)

    if not args.bot_token:
        logger.error(
            "BOT_TOKEN must be provided via --bot-token or BOT_TOKEN environment variable.")
        sys.exit(2)

    generator = TelegramInitDataGenerator(
        bot_token=args.bot_token, public_key_hex=args.public_key_hex, private_key_hex=args.private_key_hex)

    logger.info("Using bot id: %s", generator.bot_id)
    logger.info("Public key (hex): %s", generator.get_public_key_hex())

    try:
        perform_test_sequence(generator, args.base_url, args.no_wait, logger)
    except Exception as exc:
        logger.exception("Test sequence failed: %s", exc)
        sys.exit(1)


if __name__ == "__main__":
    main()

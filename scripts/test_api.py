"""
Pretty-printed logging version of the Telegram init-data generator + tester.
Everything is in English (no Russian).

Features:
- Structured logging using the standard `logging` module.
- Optional pretty console output using `rich` (falls back to plain logging).
- Clear, human-readable messages for HMAC / Ed25519 strings and computed values.
- Shortened JWT display for logs.
- Command-line switches for skipping the 60s wait and for verbose output.
- Multi-user testing with user lookup functionality.
- Handle UUID-based user IDs and hidden telegram_id/admin fields.

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
from typing import Any, Dict, Optional, Tuple

import requests

try:
    # `rich` is optional. If present we will use it for nicer console printing.
    from rich.console import Console
    from rich.logging import RichHandler
    from rich.panel import Panel
    from rich.syntax import Syntax
    from rich.table import Table
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

    If `rich` is installed, use RichHandler for nicer output. Otherwise, use a
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
    """If rich is available, print a panel. Otherwise, log normally."""
    if RICH_AVAILABLE:
        Console().print(Panel(text, title=title, expand=False))
    else:
        logger.info("%s:\n%s", title, text)


def pretty_table(title: str, data: Dict[str, Any]) -> None:
    """Display data in a table if rich is available."""
    if not RICH_AVAILABLE:
        return

    console = Console()
    table = Table(title=title, show_header=True, header_style="bold magenta")
    table.add_column("Field", style="cyan")
    table.add_column("Value", style="green")

    for key, value in data.items():
        table.add_row(str(key), str(value))

    console.print(table)


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

    def generate_realistic_user_data(self, user_id: int = 1234567890, username: str = "test_user",
                                     first_name: str = "Test", last_name: str = "User") -> Dict[str, Any]:
        return {
            "user": {
                "id": user_id,
                "first_name": first_name,
                "last_name": last_name,
                "username": username,
                "language_code": "en",
                "allows_write_to_pm": True,
                "photo_url": "https://t.me/i/userpic/320/nothing.svg",
            },
            "chat_instance": f"-{user_id}",
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


def authenticate_user(generator: TelegramInitDataGenerator, base_url: str, user_data: Dict[str, Any],
                      logger: logging.Logger, user_name: str = "User") -> Tuple[Optional[str], Optional[Dict[str, Any]]]:
    """Authenticate a user and return their JWT token and user data."""

    logger.info("\n" + "="*60)
    logger.info(f"Authenticating {user_name}")
    logger.info("="*60)

    init_data = generator.generate_init_data(user_data=user_data)

    # Extract parameters for logging
    params = user_data.copy()
    params["auth_date"] = str(int(time.time()))

    ed25519_pairs = []
    for key in sorted(params.keys()):
        value = params[key]
        if isinstance(value, (dict, list)):
            value = json.dumps(value, separators=(",", ":"), ensure_ascii=False)
        ed25519_pairs.append((key, value))
    ed25519_dcs = "\n".join([f"{k}={v}" for k, v in ed25519_pairs])

    pretty_panel(logger, f"{user_name} - Ed25519 Data Check String", ed25519_dcs)

    signature = generator.sign_ed25519(
        f"{generator.bot_id}:WebAppData\n{ed25519_dcs}")

    hmac_params = params.copy()
    hmac_params["signature"] = signature
    hmac_pairs = []
    for key in sorted(hmac_params.keys()):
        value = hmac_params[key]
        if isinstance(value, (dict, list)):
            value = json.dumps(value, separators=(",", ":"), ensure_ascii=False)
        hmac_pairs.append((key, value))
    hmac_dcs = "\n".join([f"{k}={v}" for k, v in hmac_pairs])

    pretty_panel(logger, f"{user_name} - HMAC Data Check String", hmac_dcs)

    hash_value = generator.calculate_hash(hmac_dcs)
    pretty_panel(logger, f"{user_name} - Computed Values",
                 f"Hash: {hash_value}\nSignature: {signature}")

    url_init = f"{base_url.rstrip('/')}/v1/auth/init"
    logger.info("Sending GET %s with X-InitData header (length=%d)",
                url_init, len(init_data))

    try:
        response = requests.get(
            url_init, headers={"X-InitData": init_data}, timeout=10)
    except Exception as exc:
        logger.exception("Request to %s failed: %s", url_init, exc)
        return None, None

    logger.info("/v1/auth/init -> status %d", response.status_code)

    # Print response headers (concise)
    headers_text = "\n".join(
        [f"{k}: {v}" for k, v in response.headers.items()])
    pretty_panel(logger, f"{user_name} - Response Headers", headers_text)

    cookies = response.cookies.get_dict()
    if not cookies:
        logger.warning("No cookies received from auth init response.")
        return None, None

    logger.info("Cookies: %s", json.dumps(cookies, ensure_ascii=False))

    jwt_ = cookies.get("__Host-auth_token")
    if not jwt_:
        logger.error(
            "Auth cookie '__Host-auth_token' not found. Aborting authentication.")
        return None, None

    logger.info("Received JWT (short): %s", shorten_jwt(jwt_))

    # Get user info to verify authentication
    url_me = f"{base_url.rstrip('/')}/v1/user/me"
    logger.info("Requesting user info from %s", url_me)
    try:
        response = requests.get(
            url_me, headers={"Authorization": f"Bearer {jwt_}"}, timeout=10)
    except Exception as exc:
        logger.exception("Request to %s failed: %s", url_me, exc)
        return None, None

    if response.status_code != 200:
        logger.error("/v1/user/me returned status %d", response.status_code)
        pretty_panel(logger, "User Endpoint Response", response.text)
        return None, None

    try:
        user_info = response.json().get("data")
    except Exception:
        logger.exception("Failed to parse JSON from user/me response")
        pretty_panel(logger, "Raw Response", response.text)
        return None, None

    pretty_panel(logger, f"{user_name} - Authenticated User Data", pretty_json(user_info))

    return jwt_, user_info


def lookup_user(base_url: str, jwt_token: str, target_user_uuid: str, logger: logging.Logger,
                lookup_user_name: str = "Lookup User") -> Optional[Dict[str, Any]]:
    """Look up another user by their UUID."""

    logger.info("\n" + "="*60)
    logger.info(f"User Lookup: {lookup_user_name} looking up user UUID {target_user_uuid}")
    logger.info("="*60)

    url_lookup = f"{base_url.rstrip('/')}/v1/user/{target_user_uuid}"
    logger.info("Requesting user info from %s", url_lookup)

    try:
        response = requests.get(
            url_lookup,
            headers={"Authorization": f"Bearer {jwt_token}"},
            timeout=10
        )
    except Exception as exc:
        logger.exception("Request to %s failed: %s", url_lookup, exc)
        return None

    logger.info("/v1/user/%s -> status %d", target_user_uuid, response.status_code)

    if response.status_code == 200:
        try:
            user_data = response.json().get("data")
            pretty_panel(logger, f"Found User Data (UUID: {target_user_uuid})", pretty_json(user_data))

            # Check for hidden fields
            if "telegram_id" in user_data:
                logger.warning("⚠️  telegram_id field is visible (should be hidden in user lookup)")
            if "is_admin" in user_data:
                logger.warning("⚠️  is_admin field is visible (should be hidden in user lookup)")

            return user_data
        except Exception:
            logger.exception("Failed to parse JSON from user lookup response")
            pretty_panel(logger, "Raw Response", response.text)
            return None
    else:
        logger.error("Failed to lookup user: HTTP %d", response.status_code)
        pretty_panel(logger, "Lookup Error Response", response.text)
        return None


def compare_user_profiles(own_profile: Dict[str, Any], looked_up_profile: Dict[str, Any], logger: logging.Logger) -> None:
    """Compare the user's own profile with what others see when looking them up."""

    logger.info("\n" + "="*60)
    logger.info("Profile Visibility Comparison")
    logger.info("="*60)

    # Fields that should be visible in both views
    common_fields = ["id", "first_name", "last_name", "username", "language_code", "is_banned",
                     "photo_url", "allows_write_to_pm", "created_at", "updated_at"]

    # Fields that should only be visible in own profile
    private_fields = ["telegram_id", "is_admin"]

    comparison_data = {}

    for field in common_fields:
        own_value = own_profile.get(field)
        looked_up_value = looked_up_profile.get(field)
        status = "✅" if own_value == looked_up_value else "❌"
        comparison_data[field] = {
            "own_profile": own_value,
            "looked_up": looked_up_value,
            "status": status
        }

    for field in private_fields:
        exists_in_own = field in own_profile
        exists_in_looked_up = field in looked_up_profile
        comparison_data[field] = {
            "own_profile": "PRESENT" if exists_in_own else "MISSING",
            "looked_up": "PRESENT" if exists_in_looked_up else "MISSING (good)",
            "status": "✅" if exists_in_own and not exists_in_looked_up else "❌"
        }

    # Display comparison
    if RICH_AVAILABLE:
        table = Table(title="Profile Visibility Comparison", show_header=True, header_style="bold magenta")
        table.add_column("Field", style="cyan")
        table.add_column("Own Profile", style="green")
        table.add_column("Looked Up", style="blue")
        table.add_column("Status", style="yellow")

        for field, data in comparison_data.items():
            table.add_row(
                field,
                str(data["own_profile"]),
                str(data["looked_up"]),
                data["status"]
            )

        Console().print(table)
    else:
        logger.info("Field Visibility Comparison:")
        for field, data in comparison_data.items():
            logger.info("  %s: own=%s, looked_up=%s %s",
                        field, data["own_profile"], data["looked_up"], data["status"])


def perform_test_sequence(generator: TelegramInitDataGenerator, base_url: str, no_wait: bool, logger: logging.Logger) -> None:
    """Perform the enhanced test sequence with multiple users and user lookup.

    1. Create and authenticate first user
    2. Create and authenticate second user
    3. Have second user look up first user using /v1/user/{uuid}
    4. Compare profile visibility
    5. Optionally wait for 60 seconds and test token invalidation
    """

    # User 1: Original test user
    user1_data = generator.generate_realistic_user_data(
        user_id=1234567890,
        username="test_user_1",
        first_name="Alice",
        last_name="Smith"
    )

    jwt1, user1_info = authenticate_user(generator, base_url, user1_data, logger, "User 1 (Alice)")
    if not jwt1 or not user1_info:
        logger.error("Failed to authenticate User 1. Aborting test sequence.")
        return

    user1_uuid = user1_info.get("id")
    if not user1_uuid:
        logger.error("User 1 UUID not found in response. Aborting test sequence.")
        return

    # User 2: New test user
    user2_data = generator.generate_realistic_user_data(
        user_id=9876543210,
        username="test_user_2",
        first_name="Bob",
        last_name="Johnson"
    )

    jwt2, user2_info = authenticate_user(generator, base_url, user2_data, logger, "User 2 (Bob)")
    if not jwt2:
        logger.error("Failed to authenticate User 2. Aborting test sequence.")
        return

    # User 2 looks up User 1 by UUID
    looked_up_user = lookup_user(base_url, jwt2, user1_uuid, logger, "User 2 (Bob)")

    if looked_up_user:
        logger.info("✅ User lookup successful!")

        # Compare what User 1 sees vs what User 2 sees
        compare_user_profiles(user1_info, looked_up_user, logger)

        # Verify looked up user data matches expected public fields
        expected_fields = {
            "first_name": "Alice",
            "last_name": "Smith",
            "username": "test_user_1"
        }

        mismatches = []
        for field, expected_value in expected_fields.items():
            actual_value = looked_up_user.get(field)
            if actual_value != expected_value:
                mismatches.append((field, expected_value, actual_value))

        if mismatches:
            logger.warning("⚠️  Some user data mismatches found:")
            for field, expected, actual in mismatches:
                logger.warning("   %s: expected=%r, actual=%r", field, expected, actual)
        else:
            logger.info("✅ All public user data matches expected values!")

        # Verify private fields are hidden
        if "telegram_id" not in looked_up_user:
            logger.info("✅ telegram_id correctly hidden in user lookup")
        else:
            logger.warning("⚠️  telegram_id should be hidden but is visible")

        if "is_admin" not in looked_up_user:
            logger.info("✅ is_admin correctly hidden in user lookup")
        else:
            logger.warning("⚠️  is_admin should be hidden but is visible")

    else:
        logger.error("❌ User lookup failed!")

    if no_wait:
        logger.info("Skipping token lifetime wait (--no-wait provided).")
        return

    logger.info("Waiting 60 seconds to test token expiration...")
    time.sleep(60)

    # Test token expiration for User 1
    url_init = f"{base_url.rstrip('/')}/v1/auth/init"
    init_data_user1 = generator.generate_init_data(user_data=user1_data)

    try:
        response = requests.get(
            url_init, headers={"X-InitData": init_data_user1}, timeout=10)
    except Exception as exc:
        logger.exception("Request to %s failed: %s", url_init, exc)
        return

    logger.info("Second /v1/auth/init for User 1 -> status %d", response.status_code)
    if response.status_code == 401:
        logger.info("✅ Token correctly rejected after expiry period.")
    else:
        logger.warning(
            "⚠️  Unexpected status after expiration wait: %d", response.status_code)


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

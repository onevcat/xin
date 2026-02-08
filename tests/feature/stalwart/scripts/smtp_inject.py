#!/usr/bin/env python3
import argparse
import ssl
import smtplib


def main():
    p = argparse.ArgumentParser(description="Inject an RFC822 message via SMTP into the local Stalwart harness")
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=32525)
    p.add_argument("--auth-user", required=True)
    p.add_argument("--auth-pass", required=True)
    p.add_argument("--mail-from", required=True)
    p.add_argument("--rcpt-to", action="append", required=True)
    p.add_argument("--eml", required=True, help="Path to RFC822 .eml file")
    args = p.parse_args()

    with open(args.eml, "rb") as f:
        data = f.read()

    s = smtplib.SMTP(args.host, args.port, timeout=20)
    try:
        s.ehlo()

        # Stalwart in this harness advertises STARTTLS and typically uses a self-signed cert.
        ctx = ssl.create_default_context()
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE

        s.starttls(context=ctx)
        s.ehlo()

        s.login(args.auth_user, args.auth_pass)
        s.sendmail(args.mail_from, args.rcpt_to, data)
    finally:
        try:
            s.quit()
        except Exception:
            pass


if __name__ == "__main__":
    main()

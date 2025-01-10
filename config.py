import time
from random import randint
from dotenv import load_dotenv
import os


class Config:
    def __init__(self):
        load_dotenv()
        self.DEVELOPER_KEY = os.getenv("DEVELOPER_KEY")
        self.CHANNEL_ID = os.getenv("CHANNEL_ID")
        self.TWITTER_API_KEY = os.getenv("TWITTER_API_KEY")
        self.API_SECRET_KEY = os.getenv("TWITTER_API_SECRET")
        self.ACCESS_TOKEN = os.getenv("ACCESS_TOKEN")
        self.ACCESS_SECRET_TOKEN = os.getenv("ACCESS_SECRET_TOKEN")
        self.BEARER_TOKEN = os.getenv("BEARER_TOKEN")
        self.BEARER_TOKEN = os.getenv("BEARER_TOKEN")
        self.MODE = "prod" if os.getenv("MODE") == "prod" else "dev"
        self.REQUIRED_VERIFICATIONS = 2 if self.MODE == "prod" else 0

    def get_twitter_keys(self):
        return (
            self.TWITTER_API_KEY,
            self.API_SECRET_KEY,
            self.ACCESS_TOKEN,
            self.ACCESS_SECRET_TOKEN,
            self.BEARER_TOKEN,
        )

    def wait(self, logger, max_minutes=360, min_minutes=120):
        wait_time = 5
        match self.MODE:
            case "prod":
                wait_time = randint(min_minutes * 60, max_minutes * 60)
        logger.info(f"Recheck in {self.format_time(wait_time)}")
        time.sleep(wait_time)

    def format_time(self, seconds):
        """
        Formats a number of seconds into a human-readable string.

        Args:
            seconds: The number of seconds.

        Returns:
            A human-readable string of the number of seconds.
        """

        if seconds < 0:
            seconds *= -1

        hours = int(seconds // 3600)
        minutes = int((seconds % 3600) // 60)
        milliseconds = int((seconds % 1) * 1000)
        seconds = int(seconds % 60)

        hours_string = f"{hours:02d}h:"
        minutes_string = f"{minutes:02d}min:"
        seconds_string = f"{seconds:02d}s:"
        milliseconds_string = f"{milliseconds:03d}ms"
        return f"{hours_string if hours != 0 else ''}{minutes_string if minutes != 0 else ''}{seconds_string}{milliseconds_string}"

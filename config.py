from dotenv import load_dotenv
import os


class Config:
    def __init__(self):
        load_dotenv()
        self.DEVELOPER_KEY = os.getenv("YOUTUBE_DEVELOPER_KEY")
        self.CHANNEL_ID = os.getenv("CHANNEL_ID")
        self.NEEDED_VERIFICATIONS = 1  # TODO:Increase value
        self.TWITTER_API_KEY = os.getenv("TWITTER_API_KEY")
        self.API_SECRET_KEY = os.getenv("TWITTER_API_SECRET")
        self.ACCESS_TOKEN = os.getenv("ACCESS_TOKEN")
        self.ACCESS_SECRET_TOKEN = os.getenv("ACCESS_SECRET_TOKEN")
        self.BEARER_TOKEN = os.getenv("BEARER_TOKEN")

    def get_twitter_keys(self):
        return (
            self.TWITTER_API_KEY, self.API_SECRET_KEY, self.ACCESS_TOKEN,
            self.ACCESS_SECRET_TOKEN, self.BEARER_TOKEN
        )

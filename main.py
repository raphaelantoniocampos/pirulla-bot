import sys
import logging
from datetime import datetime

from config import Config
from youtube_api import YoutubeAPI
from pirulla_bot import PirullaBot


def main() -> None:
    """
    The main entry point of the application.
    """
    logging.basicConfig(level=logging.INFO)
    logger = logging.getLogger(__name__)
    config = Config()
    developer_key = config.DEVELOPER_KEY
    channel_id = config.CHANNEL_ID
    needed_verifications = config.NEEDED_VERIFICATIONS

    youtube_api = YoutubeAPI(developer_key, channel_id)
    pirulla_bot = PirullaBot(youtube_api, needed_verifications)

    logger.info(f"Bot started: {datetime.now()}")
    while True:
        logger.info(f"Bot running: {datetime.now()}")
        pirulla_bot.start()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("Exiting due to KeyboardInterrupt")
        sys.exit()

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

    youtube_api = YoutubeAPI(config)
    pirulla_bot = PirullaBot(youtube_api, config)

    mode_log = ''
    match config.MODE:
        case 'dev':
            mode_log = 'DEV MODE\n'

    logger.info(f"{mode_log}Bot started: {datetime.now()}")
    while True:
        logger.info(f"Bot running: {datetime.now()}")
        pirulla_bot.start()
        config.wait(logger)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        logging.getLogger(__name__).info("Exiting due to KeyboardInterrupt")
        sys.exit()

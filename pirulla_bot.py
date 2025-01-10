import os
import logging

from matplotlib import pyplot as plt
import tweepy
import pandas as pd


class PirullaBot:
    def __init__(self, youtube_api, config):
        self.logger = logging.getLogger(__name__)
        self.youtube_api = youtube_api
        self.config = config

    def start(self):
        required_verifications = self.config.REQUIRED_VERIFICATIONS
        verification = 0
        while verification < required_verifications:
            stored_data = self.youtube_api.get_stored_data()
            channel_data = self.youtube_api.generate_channel_data()

            print(f"\nStored \n{stored_data.tail(5)}")
            print(f"\nChannel \n{channel_data.tail(5)}")
            differences = pd.concat([channel_data, stored_data]).drop_duplicates(
                keep=False
            )
            differences = differences.reset_index(drop=True)
            if differences.empty or differences is None:
                self.logger.info("There are no updates")
                return

            video = differences.loc[0]
            last_stored_video = stored_data.iloc[-1]

            if video["publishedAt"] < last_stored_video["publishedAt"]:
                self.logger.info("There are no updates")
                return
            self.logger.info("Seems like there are updates.")
            self.logger.info(
                f"Verifications: {verification + 1}/{required_verifications + 1}"
            )
            verification += 1
            self.config.wait(self.logger)

        self.logger.info("There are updates")
        last_video_mean = last_stored_video["currentMean"]
        current_mean = video["currentMean"]
        video_title = video["title"]
        video_duration = video["duration"]
        video_url = video["url"]
        new_channel_data = pd.concat(
            [stored_data, video.to_frame().T], ignore_index=True
        )
        self.create_variation_plot(new_channel_data)
        tweet = self.write_tweet(
            video_title, video_duration, video_url, last_video_mean, current_mean
        )

        match self.config.MODE:
            case "dev":
                self.logger.info(tweet)
            case "prod":
                self.post_tweet(tweet)
        self.youtube_api.store_data(new_channel_data)

    def create_variation_plot(self, channel_data):
        """
        Creates a plot of the variation of the Pirulla average over time and saves it at 'pirulla_plot.png'.
        """
        # Garantir que 'publishedAt' é convertido para datetime
        channel_data["publishedAt"] = pd.to_datetime(
            channel_data["publishedAt"], errors="coerce"
        )

        # Remover linhas com datas inválidas
        channel_data = channel_data.dropna(subset=["publishedAt"])

        # Preparar os dados para o gráfico
        channel_data = channel_data.sort_values(by="publishedAt")
        dates = channel_data["publishedAt"]
        means = channel_data["currentMean"] / 60

        fig = plt.figure(dpi=120, figsize=(10, 6))
        plt.grid()
        plt.plot(dates, means, c="blue")

        plt.title("Variação da cotação do Pirulla desde 2006", fontsize=20)
        fig.autofmt_xdate()
        plt.ylabel("Pirulla (min)", fontsize=14)
        plt.tick_params(axis="both", which="major", labelsize=12)

        num_ticks_y = 10
        plt.locator_params(axis="y", nbins=num_ticks_y)
        plt.savefig("./data/pirulla_plot.png", bbox_inches="tight")
        plt.close()

    def write_tweet(
        self, video_title, video_duration, video_url, last_video_mean, current_mean
    ) -> str:
        variation_time = current_mean - last_video_mean
        percentage_variation = self.get_percentage_variation(
            variation_time, last_video_mean
        )

        formated_average = self.config.format_time(current_mean)
        formated_variation_time = self.config.format_time(variation_time)
        formated_percentage_variation = self.format_percentage_variation(
            percentage_variation
        )
        formated_duration = self.config.format_time(video_duration)

        # Emojis
        underscores = (
            "_" * len(video_title) if video_title >= video_url else "_" * len(video_url)
        )
        up_emoji = "\U0001f4c8"
        down_emoji = "\U0001f4c9"
        light = "\U0001f6a8"
        camera = "\U0001f3a5"
        clock = "\U0001f552"
        link = "\U0001f517"
        time = "\U000023f3"
        bar_chart = "\U0001f4ca"

        tweet = f"""
{light} VÍDEO NOVO {light}
{camera} Título: {video_title}
{clock} Duração: {formated_duration}
{link} Url: {video_url}
{underscores}
{time} 1 Pirulla: {formated_average}
{bar_chart} Variação {up_emoji if variation_time >= 0 else down_emoji} {formated_percentage_variation} ({'+' if variation_time > 0 else '-'}{formated_variation_time})
"""
        return tweet

    def post_tweet(self, tweet):
        """
        Posts the tweet to Twitter.

        Args:
            tweet: The tweet to post.
        """

        self.logger.info("Posting tweet.")
        api_key, api_secret, access_token, access_token_secret, bearer_token = (
            self.config.get_twitter_keys()
        )

        auth = tweepy.OAuthHandler(api_key, api_secret)
        auth.set_access_token(access_token, access_token_secret)
        api = tweepy.API(auth)

        path_to_image = os.path.join(os.getcwd(), "", "./data/pirulla_plot.png")
        image_path = path_to_image
        media_id = api.media_upload(image_path)

        client = tweepy.Client(
            bearer_token, api_key, api_secret, access_token, access_token_secret
        )

        media_ids = [media_id.media_id]
        client.create_tweet(text=tweet, media_ids=media_ids)

    def get_percentage_variation(self, variation_time, last_average):
        """
        Gets the percentage variation in the time between two points in time.

        Args:
            variation_time: The variation in the time.
            last_average: The last average time.

        Returns:
            The percentage variation in the time.
        """

        return (variation_time / last_average) * 100

    def format_percentage_variation(self, percentage_variation):
        """
        Formats a percentage variation into a human-readable string.

        Args:
            percentage_variation: The percentage variation.

        Returns:
            A human-readable string of the percentage variation.
        """

        return f"{percentage_variation:.2f}%"

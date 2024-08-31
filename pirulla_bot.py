from datetime import datetime
import time
from random import randint
import os
import logging

from matplotlib import pyplot as plt
import tweepy
import pandas as pd
from config import Config


class PirullaBot:
    def __init__(self, youtube_api, needed_verifications):
        self.logger = logging.getLogger(__name__)
        self.youtube_api = youtube_api
        self.needed_verifications = needed_verifications

    def start(self):
        try:
            channel_data = pd.read_csv("channel_data.csv", sep=",")
        except FileNotFoundError:
            channel_data = self.youtube_api.generate_channel_data()
            self.youtube_api.store_data(channel_data)
            self.wait()
            return
        verifications = 0
        while verifications < self.needed_verifications:
            differences = self.check_for_new_video(channel_data)
            if differences is None:
                self.wait()
                return
            verifications += 1
            self.logger.info(f"Seems like there are updates. Verifications: {verifications}/{self.needed_verifications + 1}")
            self.wait()
            continue
        # There are updates
        self.logger.info(f"There are updates. Verifications: {verifications + 1}/{self.needed_verifications + 1}")

        while not differences.empty:
            video = differences.head(1)
            differences = differences.drop(differences.index[0]) #.reset_index(drop=True)
            last_stored_video = channel_data.tail(1)
            last_video_mean = last_stored_video['currentMean'].to_numpy()[0]
            current_mean = video['currentMean'].to_numpy()[0]
            video_title = video['title'].to_numpy()[0]
            video_duration = video['duration'].to_numpy()[0]
            video_url = video['url'].to_numpy()[0]
            channel_data = pd.concat([channel_data, video], axis=0,ignore_index=True)
            self.create_variation_plot(channel_data)
            tweet = self.write_tweet(video_title, video_duration, video_url, last_video_mean, current_mean)
            print(tweet)
            # TODO: Activate post tweet method
            # post_tweet(tweet)

        self.youtube_api.store_data(channel_data)

    def check_for_new_video(self, stored_data):
        latest_video = self.youtube_api.get_latest_video()

        if not latest_video.empty:
            new = stored_data.tail(1).compare(other=latest_video)
            if new.empty:
                return None

        channel_data = self.youtube_api.generate_channel_data()
        if stored_data.equals(channel_data):
            return None

        differences = pd.concat([channel_data, stored_data]).drop_duplicates(keep=False)

        if differences.empty:
            return None
        return differences

    def wait(self):
        wait_time = self.get_wait_time(0, 0)
        #TODO: increase wait itme
        self.logger.info(f"Recheck in {self.format_time(wait_time)}")
        time.sleep(wait_time)

    def get_wait_time(self, min_wait_time, max_wait_time):
        """
        Returns a random wait time.

        Returns:
            Between a period in minutes;
        """
        minute = 60
        return randint(min_wait_time * minute, max_wait_time * minute)

    def create_variation_plot(self, channel_data):
        """
        Creates a plot of the variation of the Pirulla average over time and saves it at 'pirulla.plot.png'

        """

        dates = []
        means = []
        for _, item in channel_data.iterrows():
            dates.append(datetime.strptime(item["publishedAt"], "%Y-%m-%dT%H:%M:%SZ"))
            means.append(item["currentMean"] / 60)

        fig = plt.figure(dpi=120, figsize=(10, 6))
        plt.grid()
        plt.plot(dates, means, c="blue")

        plt.title("Variação da cotação do Pirulla desde 2006", fontsize=20)
        fig.autofmt_xdate()
        plt.ylabel("Pirulla (min)", fontsize=14)
        plt.tick_params(axis="both", which="major", labelsize=12)

        num_ticks_y = 10
        plt.locator_params(axis="y", nbins=num_ticks_y)
        plt.savefig("pirulla_plot.png", bbox_inches="tight")

    def write_tweet(self, video_title, video_duration, video_url, last_video_mean, current_mean) -> str:
        """
        Gets the last average and the current average time and returns a formated tweet

        Args:
            last_average: The last average time.
            average_time: The current average time.

        Returns:
            A tweet that shows the variation and percentage variation with an emoji.
        """

        variation_time = self.get_variation_time(last_video_mean, current_mean)
        percentage_variation = self.get_percentage_variation(variation_time, last_video_mean)

        formated_average = self.format_time(current_mean)
        formated_variation_time = self.format_time(variation_time)
        formated_percentage_variation = self.format_percentage_variation(percentage_variation)
        formated_duration = self.format_time(video_duration)
        up_emoji = "\U0001F4C8"
        down_emoji = "\U0001F4C9"
        underscores = "_" * len(video_title) if video_title >= video_url else "_" * len(video_url)

        tweet = f"""
PIRULLABOT 2.0
AGORA CONSIDERANDO AS LIVES
ULTIMO VÍDEO
Título: {video_title}
Duração: {formated_duration}
Url: {video_url}
{underscores}
1 Pirulla: {formated_average}
Variação {up_emoji if variation_time >= 0 else down_emoji} {formated_percentage_variation} ({'+' if variation_time > 0 else '-'}{formated_variation_time})
"""
        return tweet

    def post_tweet(tweet):
        """
        Posts the tweet to Twitter.

        Args:
            tweet: The tweet to post.
        """

        print("Posting tweet.")
        config = Config()
        api_key, api_secret, access_token, access_token_secret, bearer_token = config.get_twitter_keys()

        auth = tweepy.OAuthHandler(api_key, api_secret)
        auth.set_access_token(access_token, access_token_secret)
        api = tweepy.API(auth)

        path_to_image = os.path.join(os.getcwd(), "", "pirulla_plot.png")
        image_path = path_to_image
        media_id = api.media_upload(image_path)

        client = tweepy.Client(
            bearer_token, api_key, api_secret, access_token, access_token_secret
        )

        media_ids = [media_id.media_id]
        client.create_tweet(text=tweet, media_ids=media_ids)

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

    def get_variation_time(self, last_average, new_average):
        """
        Gets the variation in the time between two points in time.

        Args:
            last_average: The last average time.
            new_average: The new average time.

        Returns:
            The variation in the time.
        """

        return new_average - last_average

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

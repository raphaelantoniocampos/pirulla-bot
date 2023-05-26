import os
from datetime import datetime
import time
from random import randint
import json

from matplotlib import pyplot as plt
from pytube import Channel
import tweepy


def get_keys():
    """
    Gets the Twitter API keys and secrets from the credentials.json file.

    Returns:
        A dictionary containing the Twitter API keys and secrets.
    """

    filename = "credentials.json"
    with open(filename) as f:
        keys = json.load(f)
        return keys


def write_tweet(last_average, average_time, last_video):
    """
    Gets the last average and the current average time and returns a formated tweet

    Args:
        last_average: The last average time.
        average_time: The current average time.

    Returns:
        A tweet that shows the variation and percentage variation with an emoji.
    """

    last_video_name = last_video.title
    last_video_url = last_video.watch_url
    variation_time = get_variation_time(last_average, average_time)
    percentage_variation = get_percentage_variation(variation_time, last_average)

    formated_average = format_time(average_time)
    formated_variation_time = format_time(variation_time)
    formated_percentage_variation = format_percentage_variation(percentage_variation)
    up_emoji = "\U0001F4C8"
    down_emoji = "\U0001F4C9"

    tweet = f"Pirulla = {formated_average}\nVariação {up_emoji if (variation_time >= 0) else down_emoji} {formated_percentage_variation} ({'+' if variation_time > 0 else '-'}{formated_variation_time})\nÚltimo vídeo: {last_video_name}\n{last_video_url}"
    return tweet


def create_variation_plot():
    """
    Creates a plot of the variation of the Pirulla average over time and saves it at 'pirulla.plot.png'

    """

    print("Plotting data.")
    filename = "channel_data.json"
    with open(filename) as f:
        data = json.load(f)

    dates = []
    averages = []
    for item in data:
        dates.append(datetime.strptime(item["Date"], "%Y-%m-%d %H:%M:%S"))
        averages.append(item["Average"] / 60)

    fig = plt.figure(dpi=120, figsize=(10, 6))
    plt.grid()
    plt.plot(dates, averages, c="blue")

    plt.title("Variação da cotação do Pirulla desde 2006", fontsize=20)
    fig.autofmt_xdate()
    plt.ylabel("Pirulla (min)", fontsize=14)
    plt.tick_params(axis="both", which="major", labelsize=12)

    num_ticks_y = 10
    plt.locator_params(axis="y", nbins=num_ticks_y)
    plt.savefig("pirulla_plot.png", bbox_inches="tight")


def post_tweet(tweet):
    """
    Posts the tweet to Twitter.

    Args:
        tweet: The tweet to post.
    """

    print("Posting tweet.")
    keys = get_keys()

    api_key = keys["API key"]
    api_secret = keys["API secret key"]
    access_token = keys["Access Token"]
    access_token_secret = keys["Access Secret Token"]
    bearer_token = keys["Bearer Token"]

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


def get_average_time(channel):
    """
    Gets the average time for a channel.

    Args:
        channel: A channel object.

    Returns:
        The average Pirulla time in seconds.
    """

    print("Verifying channel.")
    videos = list(channel.videos)
    if not videos:
        return 0
    sum_of_lengths = sum(video.length for video in videos)
    average_time = sum_of_lengths / len(videos)
    return average_time


def format_time(seconds):
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

    formated_time = ""
    hours_string = f"{hours:02d}h:"
    minutes_string = f"{minutes:02d}min:"
    seconds_string = f"{seconds:02d}s:"
    milliseconds_string = f"{milliseconds:03d}ms"
    formated_time = f"{hours_string if (hours != 0) else ''}{minutes_string if (minutes != 0) else ''}{seconds_string}{milliseconds_string}"
    return formated_time


def get_last_stats():
    """
    Gets the last stats from the channel_data.json file.

    Returns:
        A dictionary containing the last stats.
    """

    stats = {}
    filename = "channel_data.json"
    with open(filename) as f:
        content = json.load(f)
        stats = list(content)[-1]
    return stats


def save_stats(stats):
    """
    Saves the data to the channel_data.json file.

    Args:
        data: The data to save.
    """
    filename = "channel_data.json"
    with open(filename, "r") as f:
        content = json.load(f)
    content.append(stats)
    with open(filename, "w") as f:
        json.dump(content, f)


def get_variation_time(last_average, new_average):
    """
    Gets the variation in the time between two points in time.

    Args:
        last_average: The last average time.
        new_average: The new average time.

    Returns:
        The variation in the time.
    """

    variation = new_average - last_average
    return variation


def get_percentage_variation(variation_time, last_average):
    """
    Gets the percentage variation in the time between two points in time.

    Args:
        variation_time: The variation in the time.
        last_average: The last average time.

    Returns:
        The percentage variation in the time.
    """

    percentage_variation = (variation_time / last_average) * 100
    return percentage_variation


def format_percentage_variation(percentage_variation):
    """
    Formats a percentage variation into a human-readable string.

    Args:
        percentage_variation: The percentage variation.

    Returns:
        A human-readable string of the percentage variation.
    """

    return f"{percentage_variation:.2f}%"


def get_wait_time():
    """
    Returns a random wait time.

    Returns:
        Between 5 and 15 minutes in seconds;
    """
    minute = 60
    return randint(5 * minute, 15 * minute)


def main():
    channel_url = "https://www.youtube.com/channel/UCdGpd0gNn38UKwoncZd9rmA"
    channel = Channel(channel_url)

    while True:
        last_stats = get_last_stats()
        average_time = get_average_time(channel)
        last_video = list(channel.videos)[0]
        last_publish_date = last_video.publish_date
        last_average = last_stats["Average"]

        stats = {"Date": str(last_publish_date), "Average": average_time}
        # Check if there are any updates
        if (
            stats["Date"] == last_stats["Date"]
            and stats["Average"] == last_stats["Average"]
        ):
            # No updates
            print("No updates.")
        else:
            # There are updates
            print("Posting updates.")
            create_variation_plot()
            tweet = write_tweet(last_average, average_time, last_video)
            post_tweet(tweet)
            save_stats(stats)

        wait_time = get_wait_time()
        time.sleep(wait_time)


if __name__ == "__main__":
    main()

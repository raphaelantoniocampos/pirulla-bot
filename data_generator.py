import json
from pytube import Channel


def generate_channel_data(channel):
    """
    Generates all data for a channel.

    Args:
        channel: A channel object.

    Returns:
        A list of dictionaries, where each dictionary contains the date and average time for a list of videos.
    """
    videos = list(channel.videos)
    averages = []
    for average_num in range(len(videos)):
        average_data = {}
        average_sum = 0
        for i, video in enumerate(videos[::-1]):
            average_sum += video.length
            if i >= average_num:
                average_data["Date"] = str(video.publish_date)
                average_time = average_sum / (i + 1)
                average_data["Average"] = average_time
                print(f"{i + 1}/{len(videos)}: {average_data}")
                break
        averages.append(average_data)

    filename = "channel_data.json"
    with open(filename, "w") as file:
        json.dump(averages, file)


channel_url = "https://www.youtube.com/channel/UCdGpd0gNn38UKwoncZd9rmA"
channel = Channel(channel_url)
generate_channel_data(channel)

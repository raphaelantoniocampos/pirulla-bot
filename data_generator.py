import json
from pytube import Channel


def generate_channel_data(channel : Channel):
    """
    Generates all data for a channel.

    Args:
        channel: A channel object.

    Returns:
        A list of dictionaries, where each dictionary contains the date and average time for a list of videos.
    """
    videos = list(channel.videos)
    channel_data = []
    for average_num in range(len(videos)):
        data = {}
        average_sum = 0
        for i, video in enumerate(videos[::-1]):
            average_sum += video.length
            if i >= average_num:
                data["Date"] = str(video.publish_date)
                data["Last Video ID"] = video.video_id
                data["Last Length"] = video.length
                average_time = average_sum / (i + 1)
                data["Average"] = average_time
                print(f"{i + 1}/{len(videos)}: {data}")
                break
        channel_data.append(data)

    filename = "channel_data.json"
    with open(filename, "w") as file:
        json.dump(channel_data, file)


channel_url = "https://www.youtube.com/channel/UCdGpd0gNn38UKwoncZd9rmA"
channel = Channel(channel_url)
generate_channel_data(channel)

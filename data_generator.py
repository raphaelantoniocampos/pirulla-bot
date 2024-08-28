import time
import json
from pytube import Channel

from shared import channel_url


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
    # for average_num in range(100):
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

def optimized_generate_channel_data(channel: Channel):
    """
    Generates all data for a channel.

    Args:
        channel: A channel object.

    Returns:
        A list of dictionaries, where each dictionary contains the date and average time for a list of videos.
    """
    videos = list(channel.videos)
    num_videos = len(videos)
    # num_videos = 100
    channel_data = []
    average_sum = 0

    for i, video in enumerate(reversed(videos)):
        average_sum += video.length
        data = {
            "Date": str(video.publish_date),
            "Last Video ID": video.video_id,
            "Last Length": video.length,
            "Title": video.title,
            "Average": average_sum / (i + 1)
        }
        channel_data.append(data)
        time.sleep(0.2)
        print(f"{i + 1}/{num_videos}: {data}")

    filename = "op_channel_data.json"
    with open(filename, "w") as file:
        json.dump(channel_data, file)

# Função para calcular a média cumulativa dos comprimentos dos vídeos
def calculate_cumulative_mean(videos):
    # Ordenar os vídeos pela data mais antiga até a mais recente
    
    cumulative_sum = 0
    for i, video in enumerate(videos):
        print(i, video.length)
        # cumulative_sum += float(video.length)
        # cumulative_mean = cumulative_sum / (i + 1)
        # print(f"mean{i+1}: {cumulative_mean:.2f}")

channel = Channel(channel_url)
# generate_channel_data(channel)
optimized_generate_channel_data(channel)
videos = list(channel.videos)
# for i, video in enumerate(videos):
#     print(f"{i} - {video.vid_info}")
print(len(videos))
# calculate_cumulative_mean(list(channel.videos))

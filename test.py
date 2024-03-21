import paho.mqtt.client as mqtt
import time


def on_connect(client, userdata, flags, reason_code, properties):
    print(
        f"Connected with result code {reason_code} {time.time() - start_time} seconds")
    # Subscribing in on_connect() means that if we lose the connection and
    # reconnect then subscriptions will be renewed.
    client.subscribe("$SYS/#")


def on_message(client, userdata, msg):
    print(msg.topic+" "+str(msg.payload))


mqttc = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2)
mqttc.on_connect = on_connect
mqttc.on_message = on_message

start_time = time.time()
mqttc.connect("localhost", 1883, 60)

mqttc.loop_forever()

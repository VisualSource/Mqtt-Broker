import paho.mqtt.client as mqtt
import time
start = time.time()

# The callback for when a PUBLISH message is received from the server.


def on_connect(client, userdata, flags, reason_code, properties):
    print(time.time() - start)

    print(f"Connected with result code {reason_code}")
    # Subscribing in on_connect() means that if we lose the connection and
    # reconnect then subscriptions will be renewed.
    client.subscribe("test")


def on_message(client, userdata, msg):
    print(msg.topic+" "+str(msg.payload))


mqttc = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2)
mqttc.on_connect = on_connect
mqttc.on_message = on_message


mqttc.connect("localhost", 1883, 60)


mqttc.loop_forever()

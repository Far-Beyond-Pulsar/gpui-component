import subprocess
import psutil
import keyboard


def ControlContext():
    Context=["Pulsar Easy Compile Run!","Keybinds:","ctrl+r: Restart pulsar (also recompiles)","alt+c: End Pulsar+Easy run.","alt+o: stop cargo, keep Easyrun running."]
    for info in Context:
        print(info,flush=True)

Pulsar = None

def Start():
    Pulsar=subprocess.Popen("cargo run --release -p pulsar_engine",shell=True)
    ControlContext()
    return Pulsar

def StopCargo():
    for proc in psutil.process_iter(['name']):
        if proc.info['name'] == "cargo.exe":
            proc.terminate()

def Restart():
    global Pulsar
    StopCargo()
    Start()
    return Pulsar

def shutdown():
    StopCargo()

Start()

keyboard.add_hotkey("ctrl+r",Restart)
keyboard.add_hotkey("alt+o",StopCargo)
keyboard.add_hotkey("alt+c",shutdown)

keyboard.wait("alt+c")
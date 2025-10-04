import subprocess
import keyboard
import psutil

CargoCommand="cargo run --release -p pulsar_engine"
Pulsar = None
ER_Active=True

## Main Program - Handling Pulsar EXEC ##

def Start():
    Pulsar=subprocess.Popen(CargoCommand,shell=True)
    return Pulsar

def StopCargo():
    global ER_Active
    if not ER_Active: return
    for proc in psutil.process_iter(['name']):
        if proc.info['name'] == "cargo.exe":
            proc.terminate()

def Restart():
    global ER_Active
    if not ER_Active: return
    global Pulsar
    StopCargo()
    Start()
    return Pulsar

## Control Hooks ##

def SwitchActiveState():
    global ER_Active
    print("Easy-run Active Control: "+str(ER_Active))
    ER_Active=not ER_Active
    return ER_Active

def Hook_Keyboard():
    Controls={
        "ctrl+r":[Restart,"Restart Pulsar."],
        "alt+o":[StopCargo,"Close Pulsar."],
        "alt+g":[SwitchActiveState,"Toggles Easy-run Controls."]
    }
    print("Control Mappings:")
    for Key,ControlMapping in Controls.items():
        try:
            keyboard.add_hotkey(Key,ControlMapping[0])
            print(Key+":"+ControlMapping[1])
        except Exception as ControlError:
            print("Control Compile Error: "+str(ControlError))

## Main Starter ##
def Main():
    Hook_Keyboard()
    Start()
    keyboard.wait("alt+c")
    print("Easy-Runner Signing off.")
    StopCargo()

Main()
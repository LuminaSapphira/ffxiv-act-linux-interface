# ffxiv-act-linux-interface
A program that allows players of FFXIV who use Wine to play on linux to be able to parse using ACT / FFXIV Plugin.

*Project status*: On back burner during college semester.

#### Background
ACT and Wine don't really play along, especially because the FFXIV ACT plugin requires
reading the memory of the FFXIV process and intercepting packets. This program allows
using ACT in a Windows VM via emulating the memory layout of the official game client,
as well as duplicating packets sent to the game and sending them to the client running
in the VM.

---

#### How it works
`ffxiv-act-linux-interface` consists of two parts: a host and client. The host is
responsible for reading the memory of the FFXIV process running in Wine and interpreting
the relevant data, which it then sends to the client. The client creates a similar memory
layout to that found in the official client, and takes this data from the host and arranges
it accordingly. Specifically, ACT's FFXIV plugin is looking for a set of signatures that
is static in the process memory. By simply placing the same signatures into a static variable,
the memory layout is placed into the `.data` section, allowing ACT to read the process
as if it were actually FFXIV. Finally, the host also captures packets sent to the process
on linux, duplicates them, and sends them to the client in the VM (where they are promptly discarded).
The ACT plugin can then read the network data for the best accuracy.

#### How to use it
This utility is under development. At this point, it's mostly able to produce valid memory mappings, and it should be stable enough to use for FFLogs. However, it doesn't yet copy over all data, most significantly the chat log. This means you won't
be able to write triggers that depend on chat log lines. Because I only support network parsing however, you're still able
to parse accurately without it; you just won't be able to use most triggers.

1. Compile the host on linux, and the client on Windows (in your VM perhaps). 
2. Use the `config.json` files in this repo to configure the host's IP address on the client. For now, the port isn't configurable. (its 7262)
3. On the host, copy the `signatures-64.json` and `config.json` file to the application's folder. Configure the interface that FFXIV will run on for packet capture, and your computer's hostname to not double-capture packets sent to the VM. This might be automatic in the future.
4. Run the host application as root (sudo), or use the provided script to give the packet capture capability to the executable.
5. On the VM, run the client and ACT in any order. ACT should pick up the client and begin parsing.
7. Tell me all about crashes or bugs in the issue tracker.
8. Sob quietly when you realize that not all features are implemented yet.

sudo setcap cap_net_raw+ep target/release/ffxiv_act_linux_host 
sudo setcap cap_net_raw+ep target/debug/ffxiv_act_linux_host 
sudo setcap CAP_SYS_PTRACE+ep target/release/ffxiv_act_linux_host
sudo setcap CAP_SYS_PTRACE+ep target/debug/ffxiv_act_linux_host

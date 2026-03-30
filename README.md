# RustGovernor (v1.0.0)
 
An ultra-lightweight Windows power governor designed to kill fan noise and keep high-performance laptops cool during daily tasks.

I built this because Windows 11 "Balanced" and "Performance" modes are often too aggressive—spiking voltages and clocks for simple tasks like web browsing, which leads to unnecessary heat and loud fans.

The online tools just had too many sliders to manage on the UI or required manual control or just required the GUI to be open for managing thermals. Mine just works and can deliver on-demand performance if needed and is safe since it manages everything through its Windows 11 power plan named `RustGovernorPlan`.

**RustGovernor** uses a "Cool-First" logic: it keeps the CPU efficient and silent during 0-50% load and only unleashes full power when you actually floor it.

It works by taking the average load of the CPU and using it to do its logic, thus preventing any of that immediate cpu changes just because the cpu usage dropped by 1 percent that normally would cause jitter.

It creates a config.txt in its directory where its binary is located which you can edit to create a more custom granular curve if you'd like and customize it based on your system.

---

# Config

For any changes or custom configurations you can edit the config.txt located in the same directory as the binary rust-governor.exe and edit the configuration or add more parameters like this.

`dc_` or `ac_` <= this is the state in which it'll be applied AC (Wall power) or DC (Battery).

The number between state and type is the cpu usage which it'll be applied around for example `ac_15_epp=80` this will apply epp 80 when cpu usage is below 15 percent and above the value before 15 set in config.txt.

The type is the parameter in Windows 11 that it'll be applied to for example `ac_45_cooling_threshold=1` if cpu usage is above 45 it'll apply value 1 to cooling parameter in the power plan (Note: cooling thresholds are special because it sets cooling mode to 1 (Active) if cpu usage is above threshold specified).

Lastly the value after the '=' sign is the value that's going to be applied for example `ac_70_turbo=1` this will apply when the system is running on AC and  below 70 percent CPU usage above the value of the enxt parameter lower than it then set turbo to 1 (Enabled).

---

## The Results (Real-World Test)
Tested on a **Lenovo LOQ 15IAX9** (i5-12450HX, RTX 3050) while browsing in Firefox.

| Metric | Windows Default (Balanced) | RustGovernor | **Improvement** |
| :--- | :--- | :--- | :--- |
| **Peak CPU Temp** | 74°C | **54°C** | **-20°C Cooler** |
| **Average Temp** | 61°C | **52°C** | **-9°C Cooler** |
| **Peak Voltage** | 1.216V | **0.837V** | **Lower Power Draw** |
| **Max Clock Speed** | 4.38 GHz | **2.29 GHz** | **Silent Operation** |
| **Acoustics** | Fans Ramping Up | **Dead Silent** | **Peace of Mind** |

### Visual Proof (HWInfo Data)

#### 1. Windows Default Behavior (Spiky & Hot)
![Windows Thermals](screenshots/Thermals_Windows_Browsing.png)

*Notice the 1.21V spikes and the 74°C peak—totally unnecessary for just browsing.*

#### 2. RustGovernor Behavior (Stable & Cool)
![RustGovernor Thermals](screenshots/Thermals_RustGovernor_Browsing.png)

*Voltage is capped at 0.83V, and the temperature stays in the low 50s. The fans never even turned on.*

---

## Elite Performance
*   **Binary Size:** 923 KB (No bloat)
*   **RAM Usage:** Less than 1.0 MB
*   **CPU Usage:** ~0% (Written in pure Rust)
*   **Admin-Ready:** Installs as a Windows Task (Highest Privileges) to handle power schemes without UAC popups at every boot.

## 🛠️ Installation
1. Download the `rust-governor.exe`.
2. Open a terminal as Administrator.
3. cd to the folder you've downloaded `rust-governor.exe`
4. Run the file like this `./rust-governor.exe --install`.
5. It will now start automatically at boot (hidden in the background).

## 📊 Monitoring
To see exactly what the governor is doing in real-time:
```bash
rust-governor --monitor
```

---

## ⚖️ License
MIT - This is just a program I made myself to fix my temps, do whatever you want with it.

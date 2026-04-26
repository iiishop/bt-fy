<img width="1792" height="828" alt="IMG_5592" src="https://github.com/user-attachments/assets/89bef896-f9f2-4944-9bee-d8f94c4e358a" />

Kickstarter Video: https://youtu.be/o-Wq5kcca9Q​

# Butterfly Effect Installation
Matilda Nelson,
Yitong Wu,
Yuqian Lin.

## Introduction
The capacity to communicate across distance has expanded significantly, yet the experience of shared physical presence remains difficult to replicate. While messaging and video calls enable efficient exchange, they often fail to convey the feeling of another person’s presence within one’s physical space. This challenge lies at the core of Connected Environments, which explores how digital systems and IoT technologies can bridge people and places.

This project was developed within the Connected Environments Group Prototype and Pitch, focusing on communicating presence through non-screen-based interaction. Ishii and Ullmer’s concept of Tangible Bits provides a key foundation, demonstrating how digital information can be embedded in physical artefacts to support embodied interaction (Ishii & Ullmer, 1997). The project also draws on animistic design principles, using lifelike motion to evoke the presence of another person.

This project, Butterfly Effect, proposes a networked interactive installation that transforms human presence into a tangible physical signal.
### The Problem
How can presence be made tangible across distance?

## Concept
The Butterfly Effect Installation is inspired by the butterfly effect metaphor, where small actions can produce significant outcomes in interconnected systems (Lorenz, 1963). This concept is translated into an interaction design connecting two remote installations.

Each butterfly is paired with a counterpart in another location. When a user interacts with one installation, both butterflies respond simultaneously through wing motion. In addition to immediate feedback, rotation represents accumulated presence over time.

Rather than transmitting explicit information, the system enables presence to be perceived indirectly through motion. This aligns with research suggesting that non-verbal interaction can enhance emotional connection (van Erp & Toet, 2015).

## How It Works
<p align="center"> <img src="Butterfly device system layout.png" width="1000"> </p> <p align="center"> <em>Fig. 1. Butterfly device system layout</em> </p>

At Location A, a distance sensor maps proximity to wing motion. This signal is transmitted to the paired device at Location B, which replicates the motion and adds rotational feedback.

By combining real-time feedback (flapping) with longer-term representation (rotation), the system enables both immediate and accumulated expressions of presence. The modular design allows multiple butterflies to operate simultaneously.

## Design Process

### Hardware and Mechanism
<p align="center"> <img src="hardware_1.png" width="30%" /> <img src="hardware_2.png" width="30%" /> <img src="hardware_3.png" width="30%" /> </p> <p align="center"> <em>Fig. 2. Hardware</em> </p>

The system is built around the XIAO ESP32C3 microcontroller, combined with a VL53L0X/VL53L1X distance sensor and two servo motors for flapping and rotation.

<p align="center"> <img src="https://github.com/user-attachments/assets/bad917df-0edb-43a8-a31f-ad038adc3989" width="800"> </p> <p align="center"> <em>Fig. 3. Mechanism development</em> </p>

The mechanism evolved from a DC motor design to a servo-based system, improving control while requiring redesign of the enclosure.

<p align="center"> <img src="https://github.com/user-attachments/assets/4f363396-4541-494d-89bd-e356d7f14c44" style="width:30%; height:300px; object-fit:contain;" /> <img src="https://github.com/user-attachments/assets/882b2b29-88d5-48c4-8e52-59b8bc41721c" style="width:30%; height:300px; object-fit:contain;" /> <img src="https://github.com/user-attachments/assets/d92817bf-e6ee-4ccb-bb79-7bcb48281a09" style="width:30%; height:300px; object-fit:contain;" /> </p> <p align="center"> <em>Fig. 4. 3D enclosure model</em> </p>

### The app 
As the devices themselves have no screen or onboard controls, the app was needed to support setup, device management, and pairing. A browser-based method would have required users to manually join the device hotspot, open a configuration page, and then search again for the device after it reconnected to Wi-Fi with a new IP address. By handling these steps through a single mobile app, the system made setup, control, and pairing more manageable for non-technical users and more efficient during prototyping.
<p align="center">
  <img src="Add device.png" width="30%" />
  <img src="Configure Wifi.png" width="30%" />
  <img src="Control and Pair.png" width="30%" />
</p>
<p align="center">
  <em>Fig. 4. Screens from the mobile application showing device discovery, Wi-Fi provisioning, and the control/pairing interface.</em>
</p>

### System and Communication
<p align="center"> <img src="System workflow.png" width="800"> </p> <p align="center"> <em>Fig. 5. System workflow</em> </p>

The system integrates embedded software on the ESP32 and a Flutter mobile application. Devices are configured via Soft-AP, then operate in STA mode using Wi-Fi.

Communication is divided into two layers: UDP for discovery and status updates, and TCP for control and pairing. Once paired, devices exchange signals to synchronise motion.

<p align="center"> <img src="Add device.png" width="30%" /> <img src="Configure Wifi.png" width="30%" /> <img src="Control and Pair.png" width="30%" /> </p>

## Development Process
The development followed an iterative prototyping approach, where hardware, mechanical design, and communication logic were refined through testing.

Initial prototypes focused on validating presence detection and motion. A DC motor was first used but lacked precision, leading to the adoption of servo motors for controlled movement.

Distance sensing was calibrated to ensure continuous mapping between proximity and motion, improving interaction quality. Communication between paired devices was also tested and refined to support synchronised behaviour.

Integration revealed key constraints, particularly in power supply. Battery-based operation proved unreliable due to hardware limitations, leading to the use of a power bank. While this improved stability, it introduced trade-offs in weight and mechanical performance.

Overall, the system evolved through repeated cycles of testing and refinement, balancing conceptual goals with practical constraints.

### System and Communication
<p align="center"> <img src="System workflow.png" width="800"> </p> <p align="center"> <em>Fig. 5. System workflow</em> </p>

The system integrates embedded software on the ESP32 and a Flutter mobile application. Devices are configured via Soft-AP and operate in STA mode.

Communication uses UDP for discovery and TCP for control. Once paired, devices exchange signals to synchronise motion.

<p align="center"> <img src="Add device.png" width="30%" /> <img src="Configure Wifi.png" width="30%" /> <img src="Control and Pair.png" width="30%" /> </p>

### Hardware Implementation
<p align="center"> <img src="Circuit connection layout.png" width="500"> </p> <p align="center"> <em>Fig. 6. Circuit layout</em> </p>

The system uses I²C for sensing and PWM signals for servo control.

<p align="center"> <img src="Soldered circuit.jpg" width="500"> </p> <p align="center"> <em>Fig. 7. Soldered circuit</em> </p>

## Final Prototype and Evaluation
<p align="center">
  <img src="https://github.com/user-attachments/assets/b51f80df-9aa4-479b-bb22-5021ece7eaa6" width="493">
</p>
<p align="center">
  <em>Fig. 8. Final prototype</em>
</p>

The final prototype successfully integrates sensing, actuation, and communication into a compact physical device. When tested in a real-world setting, the system was able to reliably detect user presence and trigger both local and remote responses, demonstrating the core concept of translating physical activity into a perceivable signal across distance.

In terms of immediate interaction, the wing flapping behaviour proved effective. The mapping between user proximity and motion speed was clearly observable, allowing users to intuitively understand the relationship between their movement and the system’s response. This continuous mapping, rather than a simple binary trigger, contributed to a more natural and engaging interaction.

However, the performance of the rotational feedback was less consistent. Due to the limitations of the continuous servo and the constraints introduced by the external power supply, the rotation lacked precision and stability. As a result, the intended function of representing accumulated presence over time was only partially realised. While the concept was demonstrated, the clarity of this longer-term signal was reduced in practice.

From an experiential perspective, the system succeeded in creating a subtle sense of connection between two spaces. Users were able to notice activity through peripheral awareness, aligning with the project’s aim of ambient interaction. However, the interaction remained relatively minimal, and repeated exposure revealed a lack of variation in behaviour. This limited the system’s ability to convey richer or more nuanced forms of presence.

Overall, the prototype demonstrates that the core concept is technically feasible and perceptually valid, but also highlights the gap between functional implementation and expressive interaction. While presence can be translated into motion, achieving meaningful and emotionally resonant communication requires a broader range of behaviours and more refined control mechanisms.

## Challenges 
The development of the Butterfly Effect installation revealed a series of interconnected challenges that affected both technical performance and the quality of interaction.

**1. Power management**  
A primary limitation was power management. Although the system was initially designed to operate using a compact Li-ion battery, the small and fragile VBAT interface on the XIAO ESP32C3 made stable integration difficult. This led to unreliable connections and potential safety risks. As a result, the system was powered using an external power bank, which improved stability but introduced additional weight and restricted the movement of the device, particularly affecting the rotational mechanism.

**2. Mechanical precision and control**  
Another challenge was mechanical precision and control. The use of a continuous rotation servo required time-based control rather than positional feedback, leading to inconsistencies in rotation. This limited the system’s ability to accurately represent accumulated presence over time, reducing the effectiveness of one of the core interaction features.

**3. Communication and synchronisation**  
The system also faced challenges in communication and synchronisation. Maintaining real-time synchronised motion between paired devices required frequent updates, placing a significant load on the ESP32-C3. When sensing, actuation, and communication occurred simultaneously, delays or instability could arise, affecting responsiveness.

**4. Provisioning and usability**  
Additionally, the provisioning process presented usability issues. The transition from Soft-AP setup to normal Wi-Fi operation relied on the behaviour of the user’s mobile device, which could not always be controlled programmatically. In some cases, manual intervention was required to reconnect to the correct network, reducing the overall smoothness of the user experience.

These challenges highlight how hardware, software, and interaction design are tightly coupled, where limitations in one component directly influence the overall system performance.

## Improvements
Based on the identified challenges, several targeted improvements can be proposed.

**1. Improved power system**  
To address the limitations in power management, the system could integrate a dedicated battery management module and use a more robust power interface. Implementing low-power strategies such as deep sleep modes would also reduce energy consumption, enabling stable and portable operation without relying on an external power bank.

**2. Enhanced mechanical precision and control**  
To improve mechanical accuracy, the continuous rotation servo could be replaced with stepper motors or feedback-controlled servos. These alternatives would provide more precise and repeatable motion, allowing the system to more effectively represent accumulated presence over time.

**3. More efficient communication and synchronisation**  
To reduce system load, the communication strategy could be optimised by decreasing the frequency of synchronisation updates and adopting more lightweight protocols. This would maintain the perception of real-time interaction while improving system stability and responsiveness.

**4. More reliable provisioning and user experience**  
To improve usability, the provisioning process could include better reconnection logic and clearer feedback during network transitions. Enhancing the app’s ability to rediscover devices after setup would reduce the need for manual intervention and create a smoother user experience.

In addition to these technical improvements, future iterations could explore richer interaction behaviours—such as varying motion patterns, speed, or rhythm—to enhance the expressive and emotional quality of the system.

## Reflections
This project demonstrates the potential of physical computing to support remote, embodied interaction. By translating human presence into physical motion, it offers an alternative to screen-based communication and explores more ambient forms of connection.

However, a critical gap remains between conceptual ambition and technical implementation. While inspired by the butterfly effect, the system primarily indicates activity rather than fully conveying presence. The interaction successfully signals that “someone is there,” but it does not always communicate the richness or emotional nuance of that presence.

This raises a broader question about the effectiveness of minimal physical signals in expressing complex human connection. While subtlety aligns with the project’s design intention, it can also lead to ambiguity, particularly when the interaction lacks variation or contextual cues.

The project highlights the need to balance simplicity, expressiveness, and technical feasibility in Connected Environments. While reducing interaction to minimal physical cues creates a calm and ambient experience, it also limits the amount of information that can be communicated.

Despite these limitations, the project demonstrates that even simple physical interactions can create meaningful connections across distance. It provides valuable insight into how IoT systems can move beyond information exchange to support more experiential and emotionally aware forms of communication.

### Team Contributions
Wu Yitong: Hardware design and circuit integration, including wiring, assembly, system testing, and report writing.

Lin Yuqian: Software development, including the mobile app, ESP32 communication, system integration, and report writing.

Matilda Nelson: 3D modelling and fabrication, including the butterfly enclosure, laser-cut wings, video production, and report writing.

### References
	Ishii, H. and Ullmer, B. (1997) 'Tangible bits: Towards seamless interfaces between people, bits and atoms', Proceedings of the SIGCHI Conference on Human Factors in Computing Systems (CHI '97), pp. 234–241.

	Thompson, S.A., Kennedy, R., and Lomas, D. (2011) 'Ambient awareness: From random noise to digital closeness in social media', Proceedings of the SIGCHI Conference on Human Factors in Computing Systems, pp. 237–246.

	van Erp, J.B.F. and Toet, A. (2015) 'Social touch in human–computer interaction', Frontiers in Digital Humanities, 2(2), pp. 1–13.

	Lorenz, E.N. (1963) 'Deterministic nonperiodic flow', Journal of the Atmospheric Sciences, 20(2), pp. 130–141.

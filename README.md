<img width="1792" height="828" alt="IMG_5592" src="https://github.com/user-attachments/assets/89bef896-f9f2-4944-9bee-d8f94c4e358a" />

Kickstarter Video: https://youtu.be/o-Wq5kcca9Q​

# Butterfly Effect Installation #
Matilda Nelson,
Yitong Wu,
Yuqian Lin.

## Introduction ###

  The capacity to communicate at a distance has expanded enormously but the experience of shared physical presence has proven considerably harder to replicate. While messaging and video calls enable efficient exchange, they often fail to convey an important dimension of human connection: the feeling of another person’s presence within one’s physical space. This challenge sits at the heart of Connected Environments which investigates how digital systems and IoT technologies can bridge people and places across distance. This project was developed as part of the Connected Environments Group Prototype and Pitch 25/26, with the brief to design a device or service that connects people across the miles.

Within this project, we were centrally concerned with communicating physicality through non-screen-based technology. Ishii and Ullmer's concept of Tangible Bits provided a foundational reference point, establishing that digital information can be embedded in physical artefacts to engage the body and environment rather than the screen alone (Ishii & Ullmer, 1997). The project also draws on animism to translate the feeling of presence, by using the physical movement of something that looks alive to convey the sense of another person being in the room with you.

This project, Butterfly Effect, addresses presence by proposing a networked interactive installation that transforms human presence into a tangible, physical signal.

### The Problem ###

How can presence be made tangible across the miles?

## How are we doing this? ###

The Butterfly Effect installation transforms human presence into a physical, perceivable signal through a pair of networked butterfly devices. When a user approaches one device, both respond through synchronous wing movement and rotation, enabling presence in one space to be felt in another. By shifting communication from explicit information exchange to embodied environmental feedback, the system explores alternative ways of mediating connection at a distance. Research in mediated social touch suggests that non-verbal and haptic interaction can enhance emotional connectedness in remote communication (van Erp & Toet, 2015).


## Concept ###

The Butterfly Effect Installation

Inspired by the butterfly effect metaphor from chaos theory, this concept provides a theoretical framing where small initial actions can lead to disproportionate outcomes in complex systems (Lorenz, 1963). This principle is translated into interaction design and reinterpreted through remote communication: when a person moves past their installation, the butterfly on their paired device rotates creating a cumulative record of presence throughout the day. If the butterflies are all rotated, the other person has been there; if they are all aligned, they have not. When either person interacts, both butterflies flutter simultaneously sharing a live signal of connection. The butterfly is therefore positioned as both a symbol and a mechanism, through which minor everyday actions in one place generate a meaningful emotional effect elsewhere.

## How It Works ###

<p align="center">
  <img src="Butterfly device system layout.png" width="1000">
</p>
<p align="center">
  <em>Fig. 1. Butterfly device system layout</em>
</p>

The system layout(Fig. 1) presents a networked interaction between two butterfly devices located in separate spaces and connected via the internet. Each device integrates sensing, communication, and actuation to enable real-time, bidirectional interaction.

At Location A, a proximity sensor detects the user’s distance and maps it to the flapping frequency of the butterfly wings, with closer proximity resulting in faster motion. This interaction data is then transmitted to the paired device.

At Location B, the butterfly responds by synchronizing its flapping behavior and additionally provides feedback through rotation along its wall-mounted base. This tangential movement encodes the duration or accumulation of interactions over time.

The spatial separation highlights the core concept of translating local physical actions into remote, perceivable effects. By combining immediate feedback (flapping) with longer-term representation (rotation), the system enables both instant and accumulated expressions of presence. Multiple butterfly units can also be arranged on a wall to respond simultaneously, amplifying the perceived effect.

## Design Process ##

### Hardware 
<p align="center">
  <img src="hardware_1.png" width="30%" />
  <img src="hardware_2.png" width="30%" />
  <img src="hardware_3.png" width="30%" />
</p>
<p align="center">
  <em>Fig. 2. Hardware</em>
</p>

The system was built around the XIAO ESP32C3（Fig。2）, which provides both computation and wireless communication. A VL53L0X/VL53L1X（Fig. 2） time-of-flight sensor was used to detect human presence by measuring distance. Two servo motors were used for actuation: an SG92R servo(Fig. 2) drives the flapping of the butterfly wings, while an SG90-HV continuous servo provides rotational feedback to represent the duration of interaction.

All components are integrated into a 3D-printed butterfly enclosure with fabric wings, combining functional design with an expressive physical form.


### System workflow
<p align="center">
  <img src="System workflow.png" width="800">
</p>
<p align="center">
  <em>Fig. 3. System workflow</em>
</p>
During operation, the sensor continuously monitors distance. When a user is detected, the ESP32C3 triggers the local wing motion and simultaneously sends a signal via the mobile application to a paired remote device. The remote butterfly then replicates the flapping motion and rotates proportionally to the duration of presence, enabling a real-time mapping from physical presence to mechanical feedback across distance. (Fig. 3)

### The app
**[This part needs to be more specifict(Yuqian's job)]**  
The interaction logic in this system follows a clear three-stage flow: ​Users first add the device via its temporary SoftAP, ​then provide home WiFi for the device,​ and finally enter the control interface.​ After credentials are submitted, the remaining transition is handled automatically by the device, including connecting in STA mode to the target WiFi, auto-stopping SoftAP after a short delay, and bootstrapping control-related services on the local network. This design reduces onboarding friction while ensuring a smooth transition into a stable online control state.​

### The mechanism

The design of the butterfly mechanism and enclosure evolved significantly throughout the project.(Fig. 4) As an IoT device integrating both mechanical and digital components, the hardware, structure, and code were tightly coupled and continuously adapted.

<img width="1920" height="1080" alt="F6A2C7E4-8615-473A-9D37-93429B703A79" src="https://github.com/user-attachments/assets/bad917df-0edb-43a8-a31f-ad038adc3989" />
</p>
<p align="center">
  <em>Fig. 4. Designing the butterfly mechanism</em>
</p>

Initially, a DC motor was used, resulting in a hinge-based wing system driven by continuous rotation. This was later replaced by a servo motor, shifting the mechanism to angle-based movement, which required corresponding modifications to the control code.

<p align="center">
  <img src="https://github.com/user-attachments/assets/4f363396-4541-494d-89bd-e356d7f14c44"
       style="width:30%; height:300px; object-fit:contain;" />
  <img src="https://github.com/user-attachments/assets/882b2b29-88d5-48c4-8e52-59b8bc41721c"
       style="width:30%; height:300px; object-fit:contain;" />
  <img src="https://github.com/user-attachments/assets/d92817bf-e6ee-4ccb-bb79-7bcb48281a09"
       style="width:30%; height:300px; object-fit:contain;" />
</p>
<p align="center">
  <em>Fig. 5. 3D enclosure model</em>
</p>


This transition also impacted the enclosure design, as the servo had to be directly integrated into the wing assembly. The overall body therefore became more compact, with repeated redesigns to accommodate the updated mechanical and electronic layout.​ (Fig. 5)

## Development Process ##

### Wiring and soldering
<p align="center">
  <img src="Circuit connection layout.png" width="500">
</p>
<p align="center">
  <em>Fig. 6. Circuit connection layout</em>
</p>

The circuit was centered around the XIAO ESP32C3, which serves as the main controller for sensing, processing, and communication. A VL53L0X/VL53L1X distance sensor is connected via I²C (D0 as SCL, D3 as SDA) to detect human presence in real time. Two servo motors act as outputs: the SG92R servo (connected to D1) controls wing flapping, while the SG90-HV continuous servo (connected to D2) provides rotational motion to represent interaction duration. Both servos are driven by PWM signals from the microcontroller. (Fig. 6)

<p align="center">
  <img src="Soldered circuit.jpg" width="500">
</p>
<p align="center">
  <em>Fig. 7. Soldered circuit</em>
</p>

The circuit was then soldered together( Fig. 7)

### Coding **[(coding part needs to be added by Yuqian)]**  



## Final Prototype ##

<p align="center">
  <img src="https://github.com/user-attachments/assets/b51f80df-9aa4-479b-bb22-5021ece7eaa6" width="493">
</p>
<p align="center">
  <em>Fig. 8. The mechanics and enclosure</em>
</p>

The mechanics and enclosure were designed to be compact, integrating a microcontroller, two servos, a distance sensor, and a battery (which was ultimately not used), all housed beneath the wing structure.

After experimenting with paper, card, leaves, and a range of fabrics for the wings, the material needed to balance flexibility and structure: it had to be supple enough to produce a subtle “flop” or “flutter” in motion, while remaining rigid enough to hold an upright form and retain its shape. Ripstop fabric best satisfied these requirements. The wings were laser-cut from this material and subsequently heat-pressed to fix the pleats, ensuring the folds held their intended form during movement.

<p align="center">
  <img src="https://github.com/user-attachments/assets/1a164af6-4e13-40f4-90c9-f8061acefe8e" width="4032" height="2983">
</p>
<p align="center">
  <em>Fig. 9. Final prototype</em>
</p>

## Exceution of the project ##

### Hardware performance
The circuit successfully supports the main functions of the system, including wing flapping and rotational feedback. However, several practical issues were identified during testing. The first attempt to use a 3.7V 400mAh Li-ion battery (Fig. 10) to power the system was unsuccessful due to the small and closely spaced VBAT pads on the XIAO ESP32C3, which made soldering difficult and prone to short circuits. In several cases, contact between terminals caused overheating and battery damage. As a result, a power bank (Fig. 10) was used instead, providing stable and safe power.
<p align="center">
  <img src="Li-ion Battery.jpg" width="35%" />
  <img src="Power bank.jpg" width="35%" />
</p>
<p align="center">
  <em>Fig. 10. Li-ion Battery and power bank</em>
</p>

### Communication and transmission **[(Yuqian)]**  

### Overall product performance **[(can mention the rotation there) (matilda)]**  

## Challenges ## **[(coding part needs to be added by Yuqian)]**  
During development, we identified several practical challenges:​

Power management​
The butterflies rely on batteries, which discharge quickly during repeated sensing, communication, and motor actuation.​

Charging and integration​
Integrating the battery into the butterfly body is difficult because the VBAT connection on the XIAO ESP32C3 is very small and fragile, making soldering and long-term use less reliable.​

Cost reduction​
Building multiple butterfly pairs increases hardware cost, so component selection and structural simplification were important for scalability. ( components cost approx: 25 pounds in hardware alone per butterfly)​

Connection stability​
The system depends on stable communication between devices and the mobile app. Network interruptions or unstable pairing can reduce responsiveness and reliability.​

## Improvements **[(coding part needs to be added by Yuqian)]**  

Based on the current limitations, we propose several directions for future improvement:​

Improved power system​
Redesign the battery solution to increase capacity and support longer operation time, for example a “deep sleep mode”, including safer and more accessible charging methods.​

Robust hardware integration​
Develop a more reliable power connection to replace the fragile VBAT soldering, and improve internal structural design for durability.​

Cost optimization and scalability​
Simplify the hardware and fabrication process to reduce cost, enabling deployment of larger networks of butterfly pairs.​

Enhanced communication stability​
Improve the reliability of device-to-device communication and mobile app connectivity under real-world network conditions.​

Richer interaction and emotional expression​
Extend the system to encode more information (e.g., intensity, frequency, patterns) to represent different types of presence or emotional states.
​
## Reflections

In this project, a networked physical system was designed and built to transform human presence into a tangible and observable signal. The Butterfly Effect system enables users to perceive the presence of others across distance in a subtle and intuitive way. Unlike conventional digital communication, this approach emphasizes physical feedback and emotional awareness, creating a more meaningful connection between people.

While the prototype demonstrates the potential of IoT systems for remote, embodied interaction, several limitations remain. The use of a power bank, although improving reliability, introduced additional size and weight that interfered with the butterfly’s rotational movement, reducing the effectiveness of the physical interaction. Furthermore, instability caused by manual wiring highlights the limitations of ad-hoc prototyping in compact embedded systems.

These challenges reveal a trade-off between electrical reliability and mechanical performance, indicating the need for more integrated and robust design solutions in future iterations. Overall, the project shows that small physical actions can be translated into meaningful connections across distance.​

### Team Contributions
Wu Yitong: The hardware and circuit design were completed, including component integration, wiring, and assembly. System testing and debugging were carried out to identify and address issues related to servo control and power supply. The report and README were also structured and developed.

### References
	Ishii, H. and Ullmer, B. (1997) ‘Tangible bits: Towards seamless interfaces between people, bits and atoms’, Proceedings of the SIGCHI Conference on Human Factors in Computing Systems (CHI ’97), pp. 234–241.

	Thompson, S.A., Kennedy, R., and Lomas, D. (2011) ‘Ambient awareness: From random noise to digital closeness in social media’, Proceedings of the SIGCHI Conference on Human Factors in Computing Systems, pp. 237–246.

	van Erp, J.B.F. and Toet, A. (2015) ‘Social touch in human–computer interaction’, Frontiers in Digital Humanities, 2(2), pp. 1–13.

	Lorenz, E.N. (1963) ‘Deterministic nonperiodic flow’, Journal of the Atmospheric Sciences, 20(2), pp. 130–141.


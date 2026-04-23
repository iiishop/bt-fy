# Butterfly Effect Installation #

## Introduction ###
Kickstarter Video: https://youtu.be/o-Wq5kcca9Q​

As digital communication becomes increasingly pervasive, remote interaction remains largely screen-based and information-driven. While messaging and video calls enable efficient exchange, they often fail to convey a subtle yet important dimension of human connection: the feeling of another person’s presence within one’s physical space. This challenge is central to research in Connected Environments, which explores how digital systems can be embedded into everyday settings through tangible and ambient forms of interaction rather than purely visual interfaces. Ishii and Ullmer’s concept of Tangible Bits highlights how digital information can be coupled with physical artefacts to support more spatial and embodied interaction (Ishii & Ullmer, 1997), while studies on ambient awareness suggest that small, continuous signals can accumulate into a meaningful sense of social presence over time (Thompson et al., 2011).

This project, Butterfly Effect, addresses this challenge by proposing a networked interactive installation that transforms human presence into a tangible, physical signal.

## The Problem ###
Across distance, an important quality of communication is often lost: the feeling that another person is present in the same moment. Conventional messaging systems and notifications allow information to be exchanged efficiently, yet they rarely convey a sense of real-time physical presence or shared space. Existing technologies primarily prioritise explicit information exchange, leaving limited support for subtle, non-verbal aspects of connection. As a result, remote interaction often lacks the immediacy and emotional nuance associated with co-located presence.

This raises a key question for this project:

How can presence be made tangible across distance?

## Motivation ###
The Butterfly Effect installation transforms human presence into a physical, perceivable signal through a pair of networked butterfly devices. When a user approaches one device, both respond through synchronous wing movement and rotation, enabling presence in one space to be felt in another. By shifting communication from explicit information exchange to embodied environmental feedback, the system explores alternative ways of mediating connection at a distance. Research in mediated social touch suggests that non-verbal and haptic interaction can enhance emotional connectedness in remote communication (van Erp & Toet, 2015).

The concept of the “butterfly effect” provides a theoretical framing, where small initial actions can lead to disproportionate outcomes in complex systems (Lorenz, 1963). Here, this principle is translated into interaction design, allowing minor everyday actions to produce visible effects elsewhere.

## Concept ###

Concept: Butterfly Effect Installation

The project is informed by the idea of the Butterfly Effect, where a small and seemingly insignificant action can create consequences elsewhere through an interconnected system.
This concept is reinterpreted through remote communication, where a simple movement made by one person in one location is translated into a physical response in another. Human presence is sensed and converted into the motion of butterfly wings at a distant site.

The butterfly is therefore positioned as both a symbol and a mechanism, through which a minor action in one place is able to generate a meaningful emotional effect elsewhere.
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
  <em>Fig. 4.System workflow</em>
</p>
During operation, the sensor continuously monitors distance. When a user is detected, the ESP32C3 triggers the local wing motion and simultaneously sends a signal via the mobile application to a paired remote device. The remote butterfly then replicates the flapping motion and rotates proportionally to the duration of presence, enabling a real-time mapping from physical presence to mechanical feedback across distance. (Fig. 4)

### The app
The interaction logic in this system follows a clear three-stage flow: ​Users first add the device via its temporary SoftAP, ​then provide home WiFi for the device,​ and finally enter the control interface.​ After credentials are submitted, the remaining transition is handled automatically by the device, including connecting in STA mode to the target WiFi, auto-stopping SoftAP after a short delay, and bootstrapping control-related services on the local network. This design reduces onboarding friction while ensuring a smooth transition into a stable online control state.​

### The mechanism

The design of the butterfly mechanism and enclosure evolved significantly throughout the project. As an IoT device integrating both mechanical and digital components, the hardware, structure, and code were tightly coupled and continuously adapted.

Initially, a DC motor was used, resulting in a hinge-based wing system driven by continuous rotation. This was later replaced by a servo motor, shifting the mechanism to angle-based movement, which required corresponding modifications to the control code.
<p align="center">
  <img src="https://github.com/user-attachments/assets/d92817bf-e6ee-4ccb-bb79-7bcb48281a09" width="800">
</p>
<p align="center">
  <em>Fig. 5.3D enclosure model</em>
</p>


This transition also impacted the enclosure design, as the servo had to be directly integrated into the wing assembly. The overall body therefore became more compact, with repeated redesigns to accommodate the updated mechanical and electronic layout.​ (Fig. 5)

## Development Process ##

### Wiring and soldering
<p align="center">
  <img src="Circuit connection layout.png" width="500">
</p>
<p align="center">
  <em>Fig. 3.Circuit connection layout</em>
</p>

The circuit was centered around the XIAO ESP32C3, which serves as the main controller for sensing, processing, and communication. A VL53L0X/VL53L1X distance sensor is connected via I²C (D0 as SCL, D3 as SDA) to detect human presence in real time. Two servo motors act as outputs: the SG92R servo (connected to D1) controls wing flapping, while the SG90-HV continuous servo (connected to D2) provides rotational motion to represent interaction duration. Both servos are driven by PWM signals from the microcontroller. (Fig. 3)

<p align="center">
  <img src="Soldered circuit.jpg" width="500">
</p>
<p align="center">
  <em>Fig. 4.Soldered circuit</em>
</p>

The circuit was then soldered together( Fig. 4)




## Final Prototype ##
**[Important]**  Need to add the butterfly with wings, and the picture of the final product, the picture right now is not right(Matilda's job)

The mechanics and enclosure were designed to be compact, integrating a microcontroller, two servos, a distance sensor, and a battery (which was ultimately not used), all housed beneath the wing structure.

After experimenting with paper, card, leaves, and a range of fabrics for the wings, the material needed to balance flexibility and structure: it had to be supple enough to produce<img width="1920" height="1080" alt="3d model 2 butterfly" src="https://github.com/user-attachments/assets/a5f64b47-dd6a-4f20-97ac-c37b79e68127" />
 a subtle “flop” or “flutter” in motion, while remaining rigid enough to hold an upright form and retain its shape. Ripstop fabric best satisfied these requirements. The wings were laser-cut from this material and subsequently heat-pressed to fix the pleats, ensuring the folds held their intended form during movement.


## Exceution of the project ##

### Hardware performance
The circuit successfully supports the main functions of the system, including wing flapping and rotational feedback. However, several practical issues were identified during testing.






## Challenges ##
During development, we identified several practical challenges:​

Power management​
The butterflies rely on batteries, which discharge quickly during repeated sensing, communication, and motor actuation.​

Charging and integration​
Integrating the battery into the butterfly body is difficult because the VBAT connection on the XIAO ESP32C3 is very small and fragile, making soldering and long-term use less reliable.​

Cost reduction​
Building multiple butterfly pairs increases hardware cost, so component selection and structural simplification were important for scalability. ( components cost approx: 25 pounds in hardware alone per butterfly)​

Connection stability​
The system depends on stable communication between devices and the mobile app. Network interruptions or unstable pairing can reduce responsiveness and reliability.​

## Improvements

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
### References
	Ishii, H. and Ullmer, B. (1997) ‘Tangible bits: Towards seamless interfaces between people, bits and atoms’, Proceedings of the SIGCHI Conference on Human Factors in Computing Systems (CHI ’97), pp. 234–241.

	Thompson, S.A., Kennedy, R., and Lomas, D. (2011) ‘Ambient awareness: From random noise to digital closeness in social media’, Proceedings of the SIGCHI Conference on Human Factors in Computing Systems, pp. 237–246.

	van Erp, J.B.F. and Toet, A. (2015) ‘Social touch in human–computer interaction’, Frontiers in Digital Humanities, 2(2), pp. 1–13.

	Lorenz, E.N. (1963) ‘Deterministic nonperiodic flow’, Journal of the Atmospheric Sciences, 20(2), pp. 130–141.


# Butterfly Effect Installation #

## Introduction ###
Kickstarter Video: https://youtu.be/o-Wq5kcca9Q​

## The Problem ###
Across distance, an important quality of communication is often lost: the feeling that another person is present in the same moment.
Conventional messaging systems and notifications allow information to be exchanged efficiently, yet they rarely convey a sense of real-time physical presence or shared space.
This project explores the question:

How can presence be made tangible across distance?

## Motivation ###

## Concept ###

Concept: Butterfly Effect Installation
The project is informed by the idea of the Butterfly Effect, where a small and seemingly insignificant action can create consequences elsewhere through an interconnected system.
We reinterpret this concept through remote communication: a simple movement made by one person in one location triggers a physical response in another. Human presence is sensed and translated into the motion of butterfly wings at a distant site.
The butterfly therefore becomes both symbol and mechanism, representing how a minor action in one place can generate a meaningful emotional effect somewhere else.
​
## How It Works ###
Our system links two distant physical devices through a mobile application and wireless network communication.
When presence is detected at one location, the paired device responds through synchronized movement, creating a real-time connection between the two spaces.
User Scenario A person walks past their butterfly device at home. The system detects this movement and sends a signal to its paired counterpart.
Elsewhere, another person sees their butterfly begin to move, providing a subtle indication that the other person is active and present in that moment.

## Design Process ##

### Hardware 


Our system integrates sensing, computation, and actuation using
compact embedded hardware components:​

XIAO ESP32C3 (Main Controller)A compact microcontroller with
built-in Wi-Fi capability, responsible for processing sensor data and
handling communication with the mobile application.​

VL53L0X / VL53L1X Distance Sensor (Presence Detection) A time-
of-flight (ToF) sensor that measures distance with high precision,
enabling reliable detection of human presence.​

SG92R Servo Motor (Wing Flapping)A positional servo motor used
to drive the butterfly wings, simulating natural flapping motion.​

SG90-HV Continuous Servo (Rotation Feedback) A continuous
rotation servo motor used to represent interaction duration
through rotational movement.​

### Wiring
image of wiring
### The app
The interaction logic in this system follows a clear three-stage flow: ​Users first add the device via its temporary SoftAP, ​then provide home WiFi for the device,​ and finally enter the control interface.​ After credentials are submitted, the remaining transition is handled automatically by the device, including connecting in STA mode to the target WiFi, auto-stopping SoftAP after a short delay, and bootstrapping control-related services on the local network. This design reduces onboarding friction while ensuring a smooth transition into a stable online control state.​

### The mechanism

The design of the butterfly mechanism and enclosure evolved significantly throughout the project. As an IoT device with both mechanical and digital components, the hardware, structure, and code were tightly linked and had to adapt throughout.

We initially used a DC motor, which led to a hinge-based wing system driven by continuous rotation. We later moved to a servo motor, which shifted the mechanism to angle-based movement which <img width="1920" height="1080" alt="3d model 2 butterfly" src="https://github.com/user-attachments/assets/d92817bf-e6ee-4ccb-bb79-7bcb48281a09" />
<img width="1920" height="1080" alt="3d model 2 butterfly" src="https://github.com/user-attachments/assets/fe21b5f7-841f-4c61-af70-57597cb8ca52" />
<img width="1920" height="1080" alt="3d model 2 butterfly" src="https://github.com/user-attachments/assets/334ef84f-2c22-4e3d-a966-5a550d7b2f4d" />
required changes in the code.

This transition also impacted the enclosure design, as the servo had to be directly integrated into the wing assembly. The overall body therefore became more compact, with repeated redesigns to accommodate the updated mechanical and electronic layout.​

### Development Process


## Final Prototype ##

The mechanics and enclosure were designed to be compact, integrating a microcontroller, two servos, a distance sensor, and a battery (which was ultimately not used), all housed beneath the wing structure.

After experimenting with paper, card, leaves, and a range of fabrics for the wings, we found that the material needed to balance flexibility and structure: it had to be supple enough to produce<img width="1920" height="1080" alt="3d model 2 butterfly" src="https://github.com/user-attachments/assets/a5f64b47-dd6a-4f20-97ac-c37b79e68127" />
 a subtle “flop” or “flutter” in motion, while remaining rigid enough to hold an upright form and retain its shape. Ripstop fabric best satisfied these requirements. The wings were laser-cut from this material and subsequently heat-pressed to fix the pleats, ensuring the folds held their intended form during movement.

### Rotation
In the final prototype, we were removed the rotational feature of the butterflies. The continuous rotation servo had originally been intended to represent the incremental accumulation of interactions over time, allowing users to perceive their partner’s ongoing activity. However, due to power constraints, the devices could not reliably operate on battery power and instead had to remain plugged into a wired supply. This physical arrangement restricted the rotational movement and made the feature impractical to retain.

As a result, the final prototype did not fully realise our original intention of showing a partner’s accumulated presence through gradual rotational change. The rotational element had also contributed a dynamic pattern across the wall of butterflies, meaning that each installation would develop a unique visual composition based on patterns of interaction.

Although this aspect was lost, the final design still successfully communicated physical presence through movement and continued to reflect the core aim of the project: making distant presence tangible through subtle kinetic response.


### Materials to Get Started ###
## Challenges
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

In this project, we designed and built a networked physical system that transforms
human presence into a tangible, observable signal.​

The Butterfly Effect system enables users to perceive the presence of others across
distance in a subtle and intuitive way.​

Unlike conventional digital communication, our approach emphasizes physical feedback
and emotional awareness, creating a more meaningful connection between people.​

While the current prototype has limitations, it demonstrates the potential of using IoT
systems to support new forms of remote, embodied interaction.​

Small actions can create meaningful connections, even across distance.​

### Team Contributions
### References

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

The design of the butterfly mechanism and enclosure evolved a lot along the way. As this IoT device is mechanical, the design influenced the hardware and code, and vice versa, and had to adapt whenever they changed.​

### Development Process
## Final Prototype ##

The mechanics and enclosure are compact, fitting a microcontroller, two servos, a sensor, and a battery (even though we didn’t end up using it), all tucked neatly under the wings.
The wings were laser-cut from nylon fabric and then ironed to set the pleats into the correct folds.​

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
## Reflections
### Team Contributions
### References

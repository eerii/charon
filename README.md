# entities' respose 🚣

![start-export](https://github.com/eerii/charon/assets/22449369/eb9023e2-9c4f-4b20-ae4c-f8129d1a5b5e)

**entities' repose** is a traffic management game where you are tasked with guiding spirits through the rivers of the underworld. you will need to build the way for them, but soon you will find that there are **too many entities**! comment bellow how many ghosts you managed to save (^-^)

<p float="left">
  <img src="https://github.com/eerii/charon/assets/22449369/e2575f22-9262-42aa-b787-e14fa93a1fdf" width="49%" />
  <img src="https://github.com/eerii/charon/assets/22449369/711ff13c-2056-481e-953f-db34f6695474" width="49%" />
</p>

**controls:**

- left mouse / e to build a river on an empty space

- left mouse / e to delete already placed rivers

- esc to go back to the menu

*(they are remappable on the settings menu)*

**known issues:** 

none (for now)

---

### submission for the [bevy jam](https://itch.io/jam/bevy-jam-4) #4

this is my second tiny game using bevy and I am even more in love with it now than the first time. i had an awesome time working on it, and despite being away for half of the jam and not being able to do everything, i am really happy with at least having finished something.

source code at: <https://github.com/eerii/charon>

---

### using

- [bevy](https://github.com/bevyengine/bevy) game engine v0.12 (MIT, Apache 2.0)

- [bevy_asset_loader](https://github.com/NiklasEi/bevy_asset_loader) and [bevy_embedded_assets](https://github.com/vleue/bevy_embedded_assets) for assets (MIT, Apache 2.0)

- [bevy_kira_audio](https://github.com/NiklasEi/bevy_kira_audio) for audio (MIT, Apache 2.0)

- [bevy_ecs_tilemap](https://github.com/StarArawn/bevy_ecs_tilemap) (0.12 branch) for the tile system (MIT, Apache 2.0)

- [bevy_persistent](https://github.com/umut-sahin/bevy-persistent) for saving options and scores (MIT, Apache 2.0)

- [iyes_progress](https://github.com/IyesGames/iyes_progress) for the loading screen count (MIT, Apache 2.0)

- my [project template](https://github.com/eerii/hello-bevy) with ci (MIT, Apache 2.0)

- [aseprite](https://www.aseprite.org/) and [procreate](https://procreate.com/) for drawing

- [audacity](https://github.com/audacity/audacity) to create music (an attempt was made, sorry for your ears)

<details>
    <summary>todo</summary>

- [x] base systems (finde) 0.1
    - [x] compile and test builds (web, local)
    - [x] tilemap basics
    - [x] draw tiles
    - [x] spawn entities
    - [x] basic pathfinding

- [x] improved core (jue, vie) 0.2
    - [x] improved pathfinding
    - [x] bouncy collisions
    - [x] autotile shapes

- [x] mvp gameplay loop (sab mañ) 0.3
    - [x] game score
    - [x] multiple start/end points
    - [x] spawn end points
    - [x] lose timer and visual feedback

- [x] important tweaks (dom) 0.4
    - [x] add sprites
    - [x] zoom out screen
    - [x] limited path tiles
    - [x] overlay ui

- [ ] new features (lun)
    - [x] end screen (win/lose)
    - [x] restart game
    - [x] main menu with image
    - [x] music 
    - [x] tutorial text
    - [x] fullscreen
    - [x] let the player know no tiles left
    - [x] initial text when less than 30 entities 
    - [ ] sounds (ui, entities)

- [ ] playtesting and bugfixing (lun)
    - [x] review settings menu
    - [ ] profiling and optimization

- [ ] presentation (lun)
    - [x] write readme
    - [x] write jam page
    - [ ] submit game

- [ ] would be nice (???)
    - [x] animations
    - [ ] other river types + bridges
    - [ ] other spirit behaviour
    - [ ] better path drawing 
    - [ ] alternate paths
    - [ ] improve lose timer
    - [ ] spirit dialogues
    - [ ] tweening and animation
    - [ ] tiles only despawn after no entities are in them

</details>

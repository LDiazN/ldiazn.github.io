---
title: Banana split
author: Luis Diaz
tags:
  - Unity
  - C#
  - Mobile
  - Playfab
thumbnail: /assets/images/banana-split/spinning_banana.gif
description: "A simple mobile game with standard online game features: store, hard currency, soft currency, progression, scoreboard and more"
language: en
repo:
es_version: false
date: 2026-08-08
---

<table style="border-collapse: collapse; width: 100%; max-width: 440px; margin: 10px auto; top: 0;">
  <tr>
    <td style="padding: 0; border: none; vertical-align: top; width: 50%;"><img src="/assets/images/banana-split/banner.png" alt="Banana split banner" style="width: 100%; max-width: 220px; display: block;" /></td>
    <td style="padding: 0; border: none; vertical-align: top; width: 50%;"><img src="/assets/images/banana-split/gameplay.png" alt="Gameplay Example" style="width: 100%; max-width: 220px; display: block;" /></td>
  </tr>
</table>
Banana split is a mobile game where you cut bananas and avoid cutting bombs as they come and fall. The core of this demo is not the game itself but the online features it implements:


- **Highscore**: Keep track of your highest score to date
- **Leaderboard**: See the top ten high scores!
  - Note: I didn't implement display names, so for demonstration purposes I used the last 4 letters of the PlayFab ID
- **Daily Rewards**: You get a daily chest with a random provision of golden bananas (soft currency) and ruby berries  (hard currency). You can also speed up the chest by using ruby berries
- **Inventory**: You can select one of your unlocked swords in the inventory, changing the skin of the sword
- **Consumables**: You can buy consumables to help you during the game

The backend was implemented with **PlayFab**.

I made this game after my class on mobile game services in my master's degree. It was cool to implement online features so easily with PlayFab, so I spent a few days playing with Unity making this game to learn more about PlayFab. 
## Game Description

This game is heavily inspired by **fruit ninja**. You play by cutting bananas as they
appear!

You also have to avoid bombs: if you hit one without a shield, it's game
over.

Every time you miss a banana, you lose a heart. When you lose three hearts,
you lose the game.

It's an infinite game mode, try to get the highest score and collect as many
golden bananas as you can!

You have skins for your sword that change the color of the sword. 

You have two currencies:

- **Golden bananas (soft currency)**: you can get them from the random golden
  bananas that appear in the game and from the daily chest, they're used to
  buy consumables (extra shields and hearts) and some unlockables, like sword
  variations 

- **Ruby Strawberries (hard currency)**: You can only get them by paying real
  money (Not really but that's the idea!) and from the daily chest. They're used to
  buy premium unlockables

And you have two types of consumables in the game: **hearts** that heal you and **shields** that protect you from a single hit.

<video controls preload="metadata">
  <source src="/assets/videos/banana-split/gameplay.mp4" type="video/mp4">
</video>

## Implementation

The following section describes the implementation of the main online features described above.

### General architecture

The game follows a common architecture for all features:

1. Server is the authoritative source of all data, and it's defined by the `cloudscript.js` script
2. There's a `Backend.Client` class that implements all interactions with the backend from the client. It's a thin wrapper over available operations in the backend.
3. `PlayerStateManager` is the manager object that handles the current state of the player and how it's rendered to the user. Most state change operations go through this class. It also works as client-side cache, to avoid using network calls for every data lookup all the time. 
	1. The state is synced with the server after some meaningful event: Earning currency, finishing a match, purchasing something from the shop, etc
4. `PlayerData`: It's a class representing all the information related to a player. Since this can vary from version to version and it can drift between server and client at some point, we have a `Server.PlayerData` and a `Client.PlayerData` and transformation functions between each other. The server variant is used to communicate with the server, while the client variant is what the application consumes

![Banana Split architecture](/assets/images/banana-split/ArquitecturaBananaSplit.drawio.png)
### High score

- We read the `highScore` stat from PlayFab (saved to `PlayerData`), a custom stat per user, and we only overwrite it if the new score is greater. The server is source of truth, to prevent client tampering.
- When a round is over, the client checks if the cached version is lower than the new score and requests an update from the server, which also verifies it. 
<img src="/assets/images/banana-split/best_score.png" alt="Best score screen" style="width: 40%; margin: 0 auto;" />
### Leaderboard

- Implemented with PlayFab's native leaderboard API, but keyed on the `highScore` stat we mentioned before.
- To get the top 10, we just use PlayFab's `PlayFabClientAPI.GetLeaderboard` function. 
- For the name of the player we use the last 5 chars of their PlayFabId, only for demonstration purposes
<img src="/assets/images/banana-split/leaderboard.png" alt="Leaderboard screen" style="width: 40%; margin: 0 auto;" />
### Daily rewards

The daily chest is implemented by a simple timer, the `NextChestTime` variable that lives in the read only user data. It's a Unix-epoch string that specifies when the chest should be ready. 

- It's initialized to "2 minutes from now" after first login, to give the player a free chest on onboarding
- When trying to collect the reward, the server will check if it's ready by reading its internal state, not input from the client
- Upon collection, the player is granted a small random amount of hard currency and a bigger amount of soft currency, and the timer resets to +24 hours
- Players can pay hard currency to reduce time from the timer (1 hour = ruby berry) to skip the wait early

The timer that is displayed to users is just a UI gimmick; the actual state is managed by the dumb `NextChestTime` variable. The UI itself manages the countdown widget based on the value of `NextChestTime` read on initialization.


<img src="/assets/images/banana-split/timer.png" alt="Daily chest timer" style="width: 50%; margin: 0 auto;" />

### Inventory

- It's implemented using PlayFab's native inventory system
- Downloaded on start to have the currently unlocked list of objects available at any moment
- Equipped items are a local preference, this is not synced with the server. A player can equip any unlocked item in the main menu at any time
- Whenever a consumable is used or a new unlockable is unlocked, the state is synced with the server
- The **shop** is implemented entirely with PlayFab's purchasing features: `PlayFabClientAPI.PurchaseItem`, and most validation is done by PlayFab. The UI is mostly for presentation logic.

<table style="border-collapse: collapse; width: 100%; max-width: 440px; margin: 10px auto; top: 0;">
  <tr>
    <td style="padding: 0; border: none; vertical-align: top; width: 50%;"><img src="/assets/images/banana-split/inventory.png" alt="Inventory screen" style="width: 100%; max-width: 220px; display: block;" /></td>
    <td style="padding: 0; border: none; vertical-align: top; width: 50%;"><img src="/assets/images/banana-split/shop.png" alt="Shop screen" style="width: 100%; max-width: 220px; display: block;" /></td>
  </tr>
</table>

### Consumables 

Consumables are loaded into the `PlayerStateManager` on first load. Since the usage of consumables is time sensitive (taking too long can fail your round) the consumption is optimistic: if the cached value is positive, the consumable is immediately applied and a request to update it is sent to the server, the value is updated with the server's response.

![](/assets/images/banana-split/consumables.png)

The consumable consumption is implemented on top of PlayFab's inventory API. 

### Materials

The main purpose of this project was learning about PlayFab, so I didn't spend too much time on the aesthetics of the game. However, I applied several matcap materials to various items to give it a quick good look, like the golden bananas, ruby berries and the regular bananas with a cartoon matcap. 

<img src="/assets/images/banana-split/spinning_banana.gif" alt="Spinning banana" style="width: 40%; margin: 0 auto; padding: 0 0" />
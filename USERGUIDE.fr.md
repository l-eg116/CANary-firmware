# CANary - Guide Utilisateur

Le CANary est un outil autonome d'émission et de capture de trames CAN.

Dans ce guide vous apprendrez à préparer une carte Micro SD, à émettre et capturer des trames avec le CANary et à récupérer les données de la carte Micro SD.

## Sommaire

- [CANary - Guide Utilisateur](#canary---guide-utilisateur)
  - [Sommaire](#sommaire)
  - [Préparer une carte Micro SD](#préparer-une-carte-micro-sd)
  - [Capturer des trames CAN](#capturer-des-trames-can)
  - [Récupérer une capture](#récupérer-une-capture)
  - [Préparer une émission](#préparer-une-émission)
  - [Émettre des trames CAN](#émettre-des-trames-can)

## Préparer une carte Micro SD

Le CANary fonctionne avec n'importe quelle carte Micro SD acceptant le protocole SPI et formatée au format FAT.

Formatter une carte Micro SD peut se faire simplement avec l'utilitaire de formatage Windows :

<p align="center"><img src="assets/windows_FAT_formatting.png" alt="Utilitaire de formatage Windows" width="200"/></p>

Il est recommandé pour la capture de créer un dossier dédié sur la carte Micro SD afin de ne pas noyer les fichiers pour l'émission sous ceux des captures.

## Capturer des trames CAN

1. Insérez la carte Micro SD dans le CANary.

2. Allumez le CANary en le branchant via le port USB-C. Une LED clignote lorsque l'initialisation est terminée.
    > L'écran affiche les étapes de l'initialisation. Le démarrage de la carte Micro SD peut être long. Si la LED ne clignote pas après 2 minutes, l'initialisation a échoué : appuyez sur `[RESET]` ou débranchez/rebranchez le CANary. Si le problème persiste, vérifiez le formatage de la Micro SD.

3. Connectez via le connecteur dédié le CANary au bus CAN que vous voulez écouter.

4. Sélectionnez sur l'écran d'accueil du CANary l'option `Capture` puis faites `[OK]`.
    <p align="center"><img src="assets/home_screen_capture.png" alt="Home Screen - Capture" width="400"/></p>

5. Sélectionnez le dossier dans lequel vous voulez que la capture soit enregistré en naviguant la Micro SD puis faites `[OK]` pour valider.
    <p align="center"><img src="assets/file_selection_capture.png" alt="File Selection - Capture" width="400"/></p>

    > Note : si un nom de dossier est trop long, il sera raccourci et marqué d'un `~`.

6. Avant de commencer la capture, vous pouvez modifier les paramètres de celle-ci :
   - sélectionnez la Bitrate du bus avec `[UP]` et `[DOWN]` ;
   - activez le mode `Silent` avec `[RIGHT]`.
        > Le protocole CAN veut que l'envoi d'une trame sur le réseau soit validée une bit de réception. Le mode `Silent` empêche le CANary d'envoyer ce bit de réception, le rendant invisible sur le réseau CAN mais pouvant parfois empêcher le ou les émetteurs d'envoyer plus de trames.

    En haut de l'écran est affiché un rappel du dossier que vous avez sélectionné.
    <p align="center"><img src="assets/capture_standby.png" alt="Capture - Standby" width="400"/></p>

7. Appuyez sur `[OK]` pour démarrer la capture. Le clignotement de la LED s’accélère.
    <p align="center"><img src="assets/capture_running.png" alt="Capture - Running" width="400"/></p>

    > N'enlevez pas la carte SD ou ne débranchez pas le CANary pendant une capture, cela pourrait corrompre une partie de la capture ou de la carte Micro SD.

8. Appuyez de nouveau sur `[OK]` pour arrêter la capture. La LED clignote de nouveau normalement et l'écran affiche le nombre de trames capturées.
    <p align="center"><img src="assets/capture_stopped.png" alt="Capture - Stopped" width="400"/></p>

    > Si l'écran affiche de nouveau `Standby`, aucune trame n'a été capturée.

9. Pour lancer une nouvelle capture dans le même dossier, appuyez simplement de nouveau sur `[OK]`. Pour lancer une capture dans un autre dossier, appuyez sur `[LEFT]` et reprenez à l'étape 4.

## Récupérer une capture

Pour récupérer les trames capturées, éteignez (débranchez) le CANary, enlevez-en la carte Micro SD et insérez là dans un ordinateur. Vous retrouvez alors des fichiers `.log` dans le(s) dossier(s) où vous avez fait les captures.

```text
.
├── captures
│  ├── 00013790.LOG
│  ├── 00214706.LOG
│  ├── 00234989.LOG
│  ├── 00374174.LOG
│  ├── 00388492.LOG
│  └── ...
└── ...
```

Le nombre dans le nom du fichier représente l'instant où la capture à démarrer, en nombre de millisecondes depuis le démarrage du CANary. Le CANary n'ayant pas connaissance de la date, ces noms de fichiers permettent simplement de savoir dans quelle ordre les captures ont été faites. Ainsi une capture avec un nombre plus grand aura été faites après une capture avec un nombre plus petit.

Les trames contenues dans les fichiers `.log` sont au format utilisé par [`can-utils`](https://github.com/linux-can/can-utils), à savoir :

```log
(0000375767.000000) can0 001#0123456789ABCDEF
 ^^^^^^^^^┤         ^^^┤ ^^┤ ^^^^^^^^^^^^^^^┴─ 8-byte hexadecimal frame payload
          │            │   └─ 11-bit hexadecimal identifier
          │            └─ Can Interface - always can0 on a CANary
          └─ Time of capture (here in ticks since CANary boot)
```

## Préparer une émission

Pour émettre des trames CAN, des fichiers `.log` doivent préalablement être chargés sur une carte Micro SD formatée au format FAT (c.f. [Préparer une carte Micro SD](#préparer-une-carte-micro-sd)).

Les trames doivent être présentés au format utilisé par [`can-utils`](https://github.com/linux-can/can-utils) comme présenté dans la section [Récupérer une capture](#récupérer-une-capture). Les 2 premiers éléments sont ignorés et seuls les identifiants et trames sont lus.

Les fichiers doivent être encodés en UTF-8 avec des fin de ligne en LF. Le comportement du CANary n'est pas garantit en cas d'encodage différent ou de fin de ligne en CRLF. La dernière ligne du fichier doit contenir un `\n` final pour que la ligne soit considérée comme valide.

> Les 2 premiers éléments peuvent être omis du fichier `.log`, donnant le format minimal suivant :
>
> ```log
> 001#0123456789ABCDEF
> 002#23456789ABCDEF01
> 003#456789ABCDEF0123
> ...
> ```
>
> Présenté sous forme de regex, une ligne valide de LOG est interprétée ainsi :
>
> ```js
> /.* ([0-9A-F]{3})#([0-9A-F]{16})\n/i
>     ^^^^^^^^^^^^^ ^^^^^^^^^^^^^^
>      Identifier      Payload
> ```

## Émettre des trames CAN

1. Insérez la carte Micro SD dans le CANary.

2. Allumez le CANary en le branchant via le port USB-C. Une LED clignote lorsque l'initialisation est terminée.
    > L'écran affiche les étapes de l'initialisation. Le démarrage de la carte Micro SD peut être long. Si la LED ne clignote pas après 2 minutes, l'initialisation a échoué : appuyez sur `[RESET]` ou débranchez/rebranchez le CANary. Si le problème persiste, vérifiez le formatage de la Micro SD.

3. Connectez via le connecteur dédié le CANary au bus CAN sur lequel vous voulez émettre.

4. Sélectionnez sur l'écran d'accueil du CANary l'option `Emit` puis faites `[OK]`.
    <p align="center"><img src="assets/home_screen_emit.png" alt="Home Screen - Emit" width="400"/></p>

5. Sélectionnez le fichier `.log` que vous voulez envoyer sur le bus en naviguant la Micro SD puis faites `[OK]` pour valider.
    <p align="center"><img src="assets/file_selection_emission.png" alt="File Selection - Emission" width="400"/></p>

    > Note : si un nom de fichier ou dossier est trop long, il sera raccourci et marqué d'un `~`.

6. Avant d'émettre le fichier sélectionné, vous pouvez modifier les paramètres d'émission :
   - Sur l'écran principal :
     - Avec `[UP]` et `[DOWN]`, changez le nombre de répétition du fichier (entre 1 et 256). En sélectionnant `xINF`, le fichier sera répété à l'infini jusqu'à un arrêt manuel.
    <p align="center"><img src="assets/emission_standby.png" alt="Emission - Standby" width="400"/></p>

   - Sur l'écran `Emission Settings` :
        > Cet écran est accessible en appuyant sur `[RIGHT]` depuis l'écran principal. Utilisez ensuite `[UP]` et `[DOWN]` pour sélectionner un paramètre à modifier et `[RIGHT]` et `[LEFT]` pour le modifier. Appuyez enfin sur `[OK]` pour sauvegarder les modifications et retourner à l'écran principal.
     - `Bitrate` permet de choisir la Bitrate du bus CAN.
     - `Mode` permet de choisir le mode d'émission :
       - `AwaitACK` vérifie et attend le bit de réception avant d'envoyer la trame suivante.
       - `IgnoreACK` ignore le bit de réception et envoie les trames sans attendre.
       - `Loopback` lève systématiquement le bit de réception et envoie les trames sans attendre.
    <p align="center"><img src="assets/emission_settings.png" alt="Emission - Settings" width="400"/></p>

7. Appuyez sur `[OK]` pour démarrer l'émission. Le clignotement de la LED s’accélère.
    <p align="center"><img src="assets/emission_running.png" alt="Emission - Settings" width="400"/></p>

8. L'émission s'arrête automatiquement après avoir envoyé les trames le nombre de fois spécifié. Si vous aviez entré `xINF`, appuyez sur `[OK]` pour arrêter l'émission au moment désiré. La LED clignote de nouveau normalement et l'écran affiche le nombre de trames envoyées.
    <p align="center"><img src="assets/emission_stopped.png" alt="Emission - Stopped" width="400"/></p>

    > Si l'écran affiche de nouveau `Standby`, aucune trame n'a été envoyée (le fichier `.log` était vide ou au mauvais format).
    >
    > Si le nombre de trames envoyées n'est pas celui attendu, c'est que vous étiez en mode `AwaitACK` et que personne sur le bus n'a répondu ou que le fichier est au mauvais format.

9. Pour lancer une émission du même fichier, appuyez de nouveau sur `[OK]` ou modifiez les paramètres comme à l'étape 6. Pour émettre un autre fichier, appuyez sur `[LEFT]` et reprenez à l'étape 4.

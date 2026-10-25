# Schnell erfassen am Desktop

Die Desktop-App hat ein kleines Erfassen-Fenster, das über den anderen
Fenstern aufgeht. Tippe den Gedanken ein und drücke **Enter**: Er wird als
Notiz behalten, mit einem Stupser in 1 Std., und das Fenster schließt.
Die Knöpfe **10 Min.**, **30 Min.** und **1 Std.** behalten ihn
stattdessen mit diesem Stupser. **Esc** oder der Schließen-Knopf des
Fensters behält Getipptes genauso wie Enter; ist nichts getippt, schließt
das Fenster nur. Die Notiz landet in Unsortiert.

Du öffnest das Fenster

- mit dem **Tastenkürzel**, **Strg+Alt+N**, wenn du unter **Einstellungen
  › Schnell erfassen** kein anderes wählst,
- über das Menü des Tray-Symbols: **Notiz erfassen**,
- mit dem Befehl `oldbrainfix capture`, aus einem Starter, einem
  Tastenkürzel-Programm oder von einer Makro-Tastatur.

## Das Tastenkürzel auf jeder Arbeitsumgebung

Wie das Tastenkürzel wirkt, hängt von der Arbeitsumgebung ab. **Einstellungen
› Schnell erfassen** zeigt, wie es auf deiner steht.

### X11 (Xfce, Cinnamon, MATE, KDE oder GNOME unter X11, i3 und andere)

Old Brain Fix hält die Taste selbst, solange es läuft, auch im Tray. Nutzt
ein anderes Programm die Taste schon, sagen es die Einstellungen; wähle
dort eine andere.

### KDE Plasma unter Wayland

Unter Wayland kann nur die Arbeitsumgebung globale Tasten vergeben. Old
Brain Fix bittet Plasma über das GlobalShortcuts-Portal um eine und schlägt
die Taste aus seinen Einstellungen vor. Plasma fragt dich einmal, ob es
sie so übernehmen soll. Später änderst du sie unter **Systemeinstellungen
› Tastatur › Kurzbefehle**, bei Old Brain Fix; eine Änderung in den
Einstellungen von Old Brain Fix verschiebt eine Taste nicht, die Plasma
schon vergeben hat.

### GNOME unter Wayland

Neuere GNOME-Versionen bieten dasselbe Portal und fragen einmal nach der
Taste; ändern lässt sie sich in GNOMEs Tastatureinstellungen. Ältere
GNOME-Versionen haben kein Portal, und Old Brain Fix hat dort kein
Tastenkürzel: Lege selbst eines an unter **Einstellungen › Tastatur ›
Tastenkombinationen › Eigene Tastenkombinationen**, mit dem Befehl
`oldbrainfix capture`.

### sway, Hyprland und andere Wayland-Compositoren

Wo der Compositor kein GlobalShortcuts-Portal bietet, lege in seiner
Konfiguration eine Taste auf den Befehl, etwa

```text
# sway
bindsym Ctrl+Alt+n exec oldbrainfix capture
# Hyprland
bind = CTRL ALT, N, exec, oldbrainfix capture
```

### Windows und macOS

Old Brain Fix hält die Taste selbst. Ein Tray-Symbol gibt es auf diesen
Systemen noch nicht, deshalb wirkt das Tastenkürzel, solange das Fenster
der App offen ist.

### Im Vordergrund bleiben, und wohin die Tastatur danach geht

Unter X11, Windows und macOS bleibt das Erfassen-Fenster über allen
anderen Fenstern, und schließt es, geht die Tastatur zurück an das
Fenster, in dem du getippt hast, wo das System das für ein geschlossenes
Fenster tut (meistens). Unter Wayland kann eine App ihr Fenster nicht in
den Vordergrund legen: Das entscheidet der Compositor, ebenso ob das
Erfassen-Fenster die Tastatur gleich bekommt und wohin sie danach geht.
Öffnet es ohne, klicke in sein Feld; eine Fensterregel deines Compositors
kann es im Vordergrund halten.

## Die Kommandozeile

```text
oldbrainfix capture              öffnet das Erfassen-Fenster
oldbrainfix capture Milch kaufen behält „Milch kaufen“ sofort als Notiz
oldbrainfix capture -            behält, was über stdin kommt
oldbrainfix capture -- -5 kg     nach -- ist alles Text
```

Mit einem Text öffnet sich nichts, und es wird nichts ausgegeben, wenn
die Notiz behalten ist, mit einem Stupser in 1 Std. Läuft Old Brain Fix,
geht der Text an die App, sodass ihr Stupser gleich geplant ist; sonst
schreibt der Befehl die Notiz selbst, und die App findet sie beim
nächsten Start. Ohne Text öffnet sich das Erfassen-Fenster in der
laufenden App, oder die App startet mit dem Fenster und bleibt danach im
Tray.

Rückgabewerte:

| Wert | Bedeutung                                                      |
|------|----------------------------------------------------------------|
| 0    | Die Notiz ist behalten, oder das Fenster ist offen.            |
| 1    | Die Notiz ließ sich nicht behalten; der Grund steht in stderr. |
| 2    | Falsche Argumente, oder nichts zu erfassen (ein leerer Text).  |

Beispiele für Starter und Makro-Tastaturen:

- Eine Makro-Tastatur oder Maustaste: Lass ihr Programm
  `oldbrainfix capture` ausführen.
- rofi oder dmenu als einzeiliges Erfassen:
  `rofi -dmenu -p Notiz | oldbrainfix capture -`
- Ein Skript: `echo "Zahnarzt anrufen" | oldbrainfix capture -`

Unter Windows hat die App keine Konsole, gibt dort also nichts aus, und
jeder Start läuft für sich: `oldbrainfix capture` ohne Text öffnet eine
zweite Kopie der App.

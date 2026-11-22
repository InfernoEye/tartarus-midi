# Tartarus MIDI V2 — Windows 11

Most **Razer Tartarus Pro → wirtualny port MIDI**, z analogowym **Polyphonic
Aftertouch**, **velocity** i transpozycją przez fizyczny **D-pad**.

Projekt kontynuuje prototyp V1 i wykorzystuje publiczne ustalenia protokołu
HID od [ultramonaka/open-tartarus-driver](https://github.com/ultramonaka/open-tartarus-driver/blob/main/research.md),
plus bibliotekę [midir](https://github.com/Boddlnagg/midir).

**Ważne:** kod jest przygotowany na podstawie repozytorium i zrzutu działania
V1, ale nie został skompilowany ani sprawdzony na fizycznym Tartarus Pro
w tym środowisku. Testuj na Windows 11 i zgłoś logi/błędy kompilacji.

## Instalacja

1. Zainstaluj [Rust](https://rustup.rs/) (i MSVC C++ Build Tools, jeśli poprosi).
2. W [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html) utwórz
   port `Tartarus MIDI` i zostaw program uruchomiony.
3. Zamknij Razer Synapse oraz inne programy czytające ten sam Tartarus HID
   (w szczególności `open-tartarus-driver` / V1).
4. Ustaw `Tartarus MIDI` jako **MIDI input** w DAW / syntezatorze i włącz
   monitorowanie/nasłuch wejścia MIDI.
5. W PowerShell, w rozpakowanym folderze:

```powershell
cargo run --release
```

Zakończenie: `Ctrl+C`. Program wysyła wtedy Note Off dla wszystkich aktualnie
trzymanych nut i kontroler MIDI All Notes Off (CC123).

Jeżeli Twój port ma inną nazwę:

```powershell
$env:TARTARUS_MIDI_PORT = "Moja nazwa portu"
cargo run --release
```

Można też wykonać `cargo test` (testy logiki, bez urządzenia MIDI).

## MIDI

- 20 klawiszy: MIDI **60–79**; klawisz 09 to MIDI **68**.
- 1 kanał MIDI (kanał 1).
- `depth >= 100`: **Note On** (`90 nn vel`) z szacowaną velocity.
- `depth <= 80`: **Note Off** (`80 nn 00`), histereza zapobiega drganiom.
- Przy zmianie głębokości podczas trzymania: **Polyphonic Key Pressure**
  (`A0 nn pre`) z własną wartością dla każdej nuty.
- Mapowanie pressure wykorzystuje zakres **depth 80..255 → pressure 0..127**,
  żeby można było zejść z ekspresją do np. 20, nie puszczając jeszcze klawisza.
- Aftertouch jest ograniczony do co najwyżej jednego komunikatu na klawisz
  na **8 ms**, gdy wartość zmieni się o co najmniej **2** jednostki MIDI.
- Program loguje rzeczywiście wysłane komunikaty: `ON`, `AT` i `OFF`.

Przykład (wartości zależą od głębokości, velocity i tempa gry):

```text
klawisz 12 -> nuta  71 ON  vel=126
klawisz 12 -> nuta  71 AT  pre=110
klawisz 12 -> nuta  71 AT  pre=50
klawisz 12 -> nuta  71 AT  pre=20
klawisz 12 -> nuta  71 OFF
```

**Uwaga:** Tartarus mierzy **drogę klawisza**, a nie fizyczny nacisk.
Nie wszystkie syntezatory/VST obsługują wiadomości Polyphonic Key Pressure
(Aftertouch). W instrumentach, które go nie mapują na barwę/głośność/filtr,
logi AT będą widoczne, lecz dźwięk nie musi się zmieniać. Velocity nadal jest
oddzielną, przybliżoną miarą szybkości naciśnięcia.

## D-pad (Windows Raw Input)

| Kierunek | Przesunięcie |
| --- | ---: |
| ← lewo | −1 półton |
| → prawo | +1 półton |
| ↑ góra | +12 półtonów (oktawa) |
| ↓ dół | −12 półtonów (oktawa) |

**Każde wciśnięcie zmienia transpozycję o jeden krok**, bez automatycznego
powtarzania podczas przytrzymania. Transpozycja jest kumulatywna i nie znika
po puszczeniu D-pada. Log zawiera aktualne przesunięcie.

Przykład: przy zerowej transpozycji klawisz 09 = 68; po lewo = 67,
po prawo = 69 (gdy zaczniemy od zera); po górę = 80, po dół = 56.

**Ważne przy grze:** transpozycja wpływa tylko na **nowe** Note On. Jeśli
wciśniesz klawisz 09 (nuta 68), przytrzymasz go, naciśniesz ↑, a potem
puścisz klawisz 09, program wyśle `Note Off` i `AT` do **68**. To zapobiega
zawieszaniu się nut. Kolejne naciśnięcie zagra 80.

**Skąd wiadomo, że to D-pad Tartarusa?** Program odbiera Raw Input dla
klawiatur Windows, sprawdza identyfikator źródła (`VID_1532`, `PID_0244`)
i działa wyłącznie na strzałki **tego urządzenia**. Strzałki na osobnej
klawiaturze nie transponują MIDI. Nie potrzeba dodatkowego sterownika
Interception.

**Ograniczenie Win32:** Raw Input *odczytuje*, ale nie blokuje klawiszy.
Oryginalne strzałki D-pada mogą więc nadal wykonywać akcje w DAW/Windows,
np. poruszać kursorem. Do skutecznego wytłumienia tylko strzałek Tartarusa
trzeba opcjonalnego sterownika Interception lub innego filtra wejścia —
nie należy blokować wszystkich strzałek zwykłą globalną blokadą.

Transpozycja jest ograniczona do przedziału −60…+48 półtonów, tak aby
wszystkie 20 nut mieściły się w dozwolonym zakresie MIDI 0–127.

## Diagnozowanie

Jeśli AT pojawia się w konsoli, ale nie słychać zmiany dźwięku, wybierz
instrument obsługujący *Polyphonic Key Pressure* i przypisz ją do np.
cutoff filtru, poziomu głośności lub ekspresji. Wiele instrumentów obsługuje
tylko **channel aftertouch** — to inny komunikat niż używany tutaj **poly AT**.

Jeśli D-pad nie reaguje, sprawdź, czy wypisano `D-pad: Raw Input aktywny`
oraz `D-pad Raw Input: rozpoznano klawiaturę Tartarus Pro`. Bez drugiego
komunikatu interfejs keyboard nie został rozpoznany po VID/PID. Można wtedy
sprawdzić identyfikator urządzenia w Windows i dopasować filtr.

Może się też zdarzyć, że firmware wymaga innej procedury odblokowania HID;
zachowano komendę `mode 3` z działającego prototypu.

## Dostrojenie

W `src/main.rs` można zmienić:
- `NOTES`: mapę 20 klawiszy.
- `NOTE_ON_DEPTH`, `NOTE_OFF_DEPTH`: progi aktywacji/zwalniania.
- `PRESSURE_INTERVAL`: częstotliwość raportowania ekspresji.
- `PRESSURE_MIN_CHANGE`: czułość raportowania.

Projekt na GPL-3.0-or-later, wywodzący się z publicznej dokumentacji
reverse-engineeringu `open-tartarus-driver`.
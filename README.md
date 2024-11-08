# Drawing Canvas

Webbasiertes multiuser Zeichenprogramm basierend auf Eventsourcing.

Bei diesem Projekt handelt es sich um eine Abgabe für das Fach Software Systeme.  
Plain Typescript/JS für Frontend, Actix Web für Backend.  
Actix basiert auf dem Actor Model, dies wird auch für das Eventsourcing verwendet. Der Webserver ist möglichst modular aufgebaut. Das Datenbankmodell kann flexibel getauscht werden.  

## Starten
Entweder per Vite um auch Source Maps zu generieren
- `npm i`
- `npm run dev`
In einem zweiten Terminal im `/webserver` Ordner
- `cargo run --features dev`

---

Alternativ kann ein Production Build erstellt werden.
- `npm run build` Typescript muss global installiert sein, alternativ zuerst ein `npm install`
- `cargo run` 

# Abgaben:

## Blatt 6
### SPA
- Der Webserver checkt Requests die nicht an /assets/ gehen, ob diese einen speziellen Header haben
  - ist dies nicht der Fall wird der Request an das root Verzeichnis / intern redirected, der client lädt dann selbst die Ressourcen nach.
  - alternativ könnte der Webserver hier auch mit einer hydrierten gerenderten Seite bereits antworten 
  - ist der Header gesetzt, werden nur die benötigten Teile der HTML Seite zurückgegeben 
  - Der Nachteil an der aktuellen Implementierung ist das URL Query Handling. Da keine Querys in der Anwendung verwendet werden, wurde hier kein robustes System entwickelt. (Redirects beachten keine Query Parameter)
- Die Funktion orientiert sich an der Bibliothek HTMX und dem Konzept der Hypermedia Driven Applications
  - https://htmx.org/essays/hypermedia-driven-applications/
- Der Websocket Ansatz wurde nicht gewählt, da die SPA bereits für Blatt 4 entwickelt wurde und ein Umbau mehr Zeit gekostet hätte :) (zumal Websocket Handling in Actix etwas komplexer ist als in anderen Sprachen/Frameworks)
  - Jedoch wäre die Implementierung über Websockets wesentlich einfacher gewesen

### Canvas Websocket
- Der Websocket sendet die gleichen Events wie der Client intern selbst auch
- Der Canvas besitzt einen eigenen Websocket, mit dem sich die Clients verbinden 
- Die Anwendung erlaubt einem User mehrere Sessions aktiv zu betreiben
  - jede Session wird wie ein eigener User behandelt (keine shared selection)
- Die Anwendung persistiert alle Daten
  - Hier wurde ein simples event sourcing basiertes System gebaut
  - jedoch sehr simpel (kein versioning oder rollbacks etc.)
  - die Events dienen hauptsächlich einem einfachen Laden und Speichern des Zustandes
  - Beim Laden der Events werden diese sporadisch validiert
    - es gibt keine selected shapes oder connected users, nachdem der Server mit Ctrl+C beendet wurde
- Die Anwendung überwacht die Rechte aller Benutzer
  - Rechte können live geändert werden
- Wird ein Event vom Server nicht akzeptiert, wird das dem Client nicht mitgeteilt
  - Dieser desynchronisiert dann (der "offizielle" Client sendet jedoch nie Events, wenn er nicht darf)
  - Es gibt keinen Mechanismus, mit dem der Client den Zustand des Canvas mit dem Server vergleichen kann

### JWT
- Das Arbeiten mit den JWT als Session Storage ist umständlich, diese sind auch nicht als solche gedacht
- Dennoch wurde starker Gebrauch von ihnen gemacht, die Anwendung speichert allen State in den JWT
  - Nur der erste User der ein Canvas öffnet, löst im Websocket Server ein Laden in der "DB" (CanvasStore) aus
- Das Token wird alle 30 Sekunden invalidiert
  - der Refresh passiert automatisch in einer Middleware
  - Aktuell wird ein Refresh immer erlaubt
    - Das ist schlecht, eigentlich sollte hier ein Verfahren angewandt werden, um zu überprüften, ob der Refresh erlaubt ist oder nicht. Jedoch für diese Anwendung zu viel Aufwand und nicht Teil der Aufgabe
- **Das heißt, es kann unter Umständen 30-60 Sekunden dauern, bis die Anwendung auf eine Anfrage reagiert bzw. diese nicht abblockt**

### Rust
- Für den Server wurde Rust verwendet
- Hierbei wurde der Speicher nicht optimiert
  - viele copy :)  
- Error Handling ist eher schlecht
  - Keine Custom Errors oder Error pages 

## Blatt 4
### Benutzerverwaltung
- Passwort verwendet Argon2, mit passend sicheren Parametern
  - benötigt kein extra Salting, da Argon2 das bereits macht
  - Es wurde kein zusätzlicher Pepper verwendet
- Benutzerverwaltung wird auch durch Eventsourcing abgebildet
- Cookie wird mit SameSite=Lax gesetzt und httponly
  - SameSite=Lax schützt vor CSRF bei POST Requests von anderen Seiten
  - Alle Endpunkte werden mit POST Requests aufgerufen
  - Es wird kein extra CSRF Token implementiert
    - https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html#token-based-mitigation
  - XSS ist kein Problem, da nur sehr wenig userconent angezeigt wird (ggf. Canvasname und Username) und das escaped wird
- JWT
  - https://curity.io/resources/learn/jwt-best-practices
  - http://cryto.net/~joepie91/blog/2016/06/19/stop-using-jwt-for-sessions-part-2-why-your-solution-doesnt-work/
  - ist eigentlich nicht für Session Storage gedacht
    - nicht einfach ein JWT Token zu entziehen, bzw. verliert das JWT Format ein wenig den Sinn, wenn man troztdem für jede Anfrage in der db testet ob das Token Valid ist.
    - JWT ist besser für single shot authentifizierung und authorisierung
  - Problem, rechteupdates werden nicht in JWT wiedergespiegelt, wenn diese nicht erneuert wird.
    - Lösung: TBD

## Blatt 3
### Event Sourcing
- Shapes sind nun einfache reine Objekte
  - Ermöglicht einfaches Serialisieren und Deserialisieren


- Jede Komponente speichert selbst die Shapes
  - `ShapeStore` erleichtert das Speichern und Verwalten der Shapes
  - Canvas besitzt spezielle Shapes die Rendering Informationen enthalten
  - SelectionTool könnte auch eigene Shapes bekommen (aus Zeitgründen verwendet is jedoch die CanvasShapes)


- Alle Events und Handler sind strict typed
  - Das hat einiges an Zeit gekostet schöne Typen zu bauen, aber wenn es dann funktioniert macht es wirklich Spaß :)
  - Manche Typen funktionieren noch nicht so wie ich mir das vorstelle, aber das ist ein Lernprozess
  
- Aus Gründen der Separation of Concerns kann der Canvas nun selbst entscheiden, wann er ein redraw durchführt 
  - Dafür wird der RequestAnimationFrame Mechanismus verwendet, wenn sich etwas an den Shapes ändert, dadurch werden je nach Refreshrate des Browsers mehrere Events gebündelt. Hier ist mir die simplizität wichtiger als die Performance.


- Die Events besitzen ein `origin` Feld
  - Mit diesem kann die Komponente entscheiden, ob sie auf ein Event Reagieren muss.
  - So ist es möglich die Events verschiedener Clients zu synchronisieren
  - Oder beispielsweise mehrere `SelectionTools` zu verwenden, die sich automatisch synchronisieren


- Änderungen an den Shapes und auch Bewegung wird durch das `ShapeChanged` Event abgebildet
  - Das ermöglicht es einfacher Snapshots zu erstellen, es muss nur das letzte ShapeChanged Event gespeichert werden
    - Für Shanpshots muss noch eine Lösung für das Z-Indexing gefunden werden.
    - Gelöschte Elemente aus dem Snapshot zu entfernen würde Z-Change Events durcheinander bringen
  - Zudem gibt es für alle Zustandsänderungen nur ein Event


## Blatt 2
### Meine Lösung für Z - Index
Beibehalten des Arrays.  
Wird die Z-Order geändert, werden die Elemente geshifted und wieder eingefügt.  
Mithilfe eines Index Lookup Caches, müssen die Shapes nicht in der Liste gesucht werden.  
**Ohne genauen Beweis sollte das für das reine Ändern der Reihenfolge eine Array größen unabhängige Laufzeit sein.** O(k) wobei k die Anzahl der Layer ist.

**Problem:** Ein Element an den Anfang zu fügen ist teuer. Je nach Implementierung der JS-Runtime. V8 verwendet wohl einen copy bei jedem unshift.  
Die Optimierung bringt nicht viel, da die meiste Zeit/Rechenleistung beim Rendern der Shapes verbraucht wird.


### Alternative überlegung - Linked List
Erste Idee war es eine Linked List zu verwenden, um einfach die Glieder einzufügen und von O(1) Laufzeit beim Einfügen zu profitieren.  
Shapes werden in der Anwendung so oder so meist iteriert, ein Zugriff von O(n) für das Suchen / direkte Aufrufen wäre also nicht so schlimm.

**Problem:** Implementierung umständlicher als Array Ansatz. Tauschen an sich sogar langsamer als mit Array, da Element erst gesucht werden muss, aber auch hier wäre ein Index Lookup Cache möglich.  
Bringt auch keine Vorteile, da die meiste Zeit/Rechenleistung beim Rendern der Shapes verbraucht wird.

###  Alternative überlegung - B-Tree
Verwendung eines Baums, erlaubt schnelles Einfügen und dadurch schnelles Ändern des Z-Index O(log n).  

**Vorteil:** Zu Knoten kann ein rendering Cache gespeichert werden. Idealerweise wird ein selbst balancierender Baum der mehrere Kinder pro Knoten erlaubt verwendet.
**Problem:** Aufwendige und komplexe Implementierung.



### Quellen
- https://www.bigocheatsheet.com/
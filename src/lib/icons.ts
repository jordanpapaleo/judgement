// The app makes no network calls, so Iconify can't fetch icons on demand.
// Register the bundled Unicons set (the same `uil:*` names Helm uses) up front.
import { addCollection } from "@iconify/react";
import icons from "@iconify-json/uil/icons.json";

addCollection(icons);

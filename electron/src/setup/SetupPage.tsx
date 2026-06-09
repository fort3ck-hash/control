import { Topbar } from "@/components/Topbar";
import React from "react";

export function SetupPage() {
  return (
    <Topbar
      pathname="/_sidebar/setup"
      items={[
        {
          link: "machines",
          activeLink: "machines",
          title: "Maschinen",
          icon: "lu:Factory",
        },
        {
          link: "ethercat",
          activeLink: "ethercat",
          title: "EtherCat",
          icon: "lu:EthernetPort",
        },
        {
          link: "update/choose-version",
          activeLink: "update",
          title: "Aktualisierung",
          icon: "lu:CircleFadingArrowUp",
        },
        {
          link: "troubleshoot",
          activeLink: "troubleshoot",
          title: "Fehlersuche",
          icon: "lu:MessageSquareWarning",
        },
        {
          link: "metrics",
          activeLink: "metrics",
          title: "Metriken",
          icon: "lu:Activity",
        },
      ]}
    />
  );
}

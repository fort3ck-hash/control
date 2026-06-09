import { Page } from "@/components/Page";
import React from "react";
import { useWagoPower1 } from "./useWagoPower1";
import { ControlGrid } from "@/control/ControlGrid";
import { ControlCard } from "@/control/ControlCard";
import { TimeSeriesValueNumeric } from "@/control/TimeSeriesValue";
import { SelectionGroup } from "@/control/SelectionGroup";
import { Mode } from "./wagoPower1Namespace";

export function WagoPower1ControlPage() {
  const {
    state,

    voltage,
    current,

    setMode,

    isLoading,
    isDisabled,
  } = useWagoPower1();

  return (
    <Page>
      <ControlGrid columns={2}>
        <ControlCard title="Power">
          <TimeSeriesValueNumeric
            label="Voltage"
            unit="V"
            timeseries={voltage}
            renderValue={(value) => value.toFixed(2)}
          />
          <TimeSeriesValueNumeric
            label="Current"
            unit="mA"
            timeseries={current}
            renderValue={(value) => value.toFixed(2)}
          />
        </ControlCard>
        <ControlCard title="Modus">
          <SelectionGroup<Mode>
            value={state?.mode}
            disabled={isDisabled}
            loading={isLoading}
            onChange={setMode}
            orientation="vertical"
            className="grid h-full grid-cols-2 gap-2"
            options={{
              Off: {
                children: "Aus",
                icon: "lu:PowerOff",
                isActiveClassName: "bg-gray-600",
                className: "h-full",
              },
              On24V: {
                children: "Ein",
                icon: "lu:Power",
                isActiveClassName: "bg-green-600",
                className: "h-full",
              },
            }}
          />
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}

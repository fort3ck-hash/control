import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import React from "react";
import { ControlGrid } from "@/control/ControlGrid";
import { SelectionGroupBoolean } from "@/control/SelectionGroup";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { useTestMotor } from "./useTestMotor";

export function TestMotorControlPage() {
  const { state, setMotorOn, setVelocity } = useTestMotor();

  // Fallback in case state is still null
  const safeState = state ?? { motor_enabled: false, motor_velocity: 0 };

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Basic control */}
        <ControlCard title="Motorstatus">
          {/* On/Off switch */}
          <Label label="Leistungsstatus">
            <SelectionGroupBoolean
              value={safeState.motor_enabled}
              // Icon mapping for True/False
              optionTrue={{ children: "Aktiviert", icon: "lu:Play" }}
              optionFalse={{ children: "Deaktiviert", icon: "lu:CirclePause" }}
              onChange={(val) => setMotorOn(val)}
            />
          </Label>
        </ControlCard>

        {/* Velocity */}
        <ControlCard title="Einstellungen">
          {/* Velocity input with unit */}
          <Label label="Zielgeschwindigkeit">
            <EditValue
              title="Geschwindigkeit"
              value={safeState.motor_velocity}
              unit="rpm"
              min={0}
              max={1000} // Limit
              step={1}
              onChange={(val) => setVelocity(val)}
              // Displays the value (as integer)
              renderValue={(v) => v.toFixed(0)}
            />
          </Label>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}

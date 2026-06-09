import React, { useState } from "react";
import { Page } from "@/components/Page";
import { ControlCard } from "@/control/ControlCard";
import { Label } from "@/control/Label";
import { SelectionGroupBoolean } from "@/control/SelectionGroup";
import { EditValue } from "@/control/EditValue";
import { roundToDecimals } from "@/lib/decimal";
import { useExtruder2 } from "./useExtruder";
import { ControlGrid } from "@/control/ControlGrid";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export function Extruder2SettingsPage() {
  const {
    state,
    defaultState,
    setInverterRotationDirection,
    resetInverter,
    setExtruderPressureLimit,
    setExtruderPressureLimitEnabled,
    setInverterTargetPressure,
    setPressurePidKp,
    setPressurePidKi,
    setPressurePidKd,
    setTemperaturePidValue,
    setTemperatureTargetEnabled,
    startPressurePidAutoTune,
    stopPressurePidAutoTune,
  } = useExtruder2();

  const [showAdvanced, setShowAdvanced] = useState(false);
  const [tuneDelta, setTuneDelta] = useState(1.0);
  const [frequencyStepHz, setFrequencyStepHz] = useState(2.5);

  return (
    <Page>
      <ControlCard className="bg-red" title="Umrichter-Einstellungen">
        <Label label="Drehrichtung">
          <SelectionGroupBoolean
            value={state?.rotation_state.forward}
            optionTrue={{ children: "Vorwaerts" }}
            optionFalse={{ children: "Rueckwaerts" }}
            onChange={setInverterRotationDirection}
          />
        </Label>

        <Label label="Umrichter zuruecksetzen">
          <button
            onClick={resetInverter}
            className="inline-block w-fit max-w-max rounded bg-red-600 px-4 py-4 text-base whitespace-nowrap text-white hover:bg-red-700"
            style={{ minWidth: "auto", width: "fit-content" }}
          >
            Reset Inverter
          </button>
        </Label>
      </ControlCard>

      <ControlCard className="bg-red" title="Extruder-Einstellungen">
        <Label label="Duesen-Druckgrenze">
          <EditValue
            value={state?.extruder_settings_state.pressure_limit}
            defaultValue={defaultState?.extruder_settings_state.pressure_limit}
            unit="bar"
            title="Duesen-Druckgrenze"
            min={0}
            max={350}
            renderValue={(value) => roundToDecimals(value, 0)}
            onChange={setExtruderPressureLimit}
          />
        </Label>
        <Label label="Duesen-Druckgrenze aktiviert">
          <SelectionGroupBoolean
            value={state?.extruder_settings_state.pressure_limit_enabled}
            optionTrue={{ children: "Aktiviert" }}
            optionFalse={{ children: "Deaktiviert" }}
            onChange={setExtruderPressureLimitEnabled}
          />
        </Label>
        <Label label="Duesen-Temperatursollwert aktiviert">
          <SelectionGroupBoolean
            value={
              state?.extruder_settings_state.nozzle_temperature_target_enabled
            }
            optionTrue={{ children: "Aktiviert" }}
            optionFalse={{ children: "Deaktiviert" }}
            onChange={setTemperatureTargetEnabled}
          />
        </Label>
        <Label label="Erweiterte PID-Einstellungen anzeigen">
          <SelectionGroupBoolean
            value={showAdvanced}
            optionTrue={{ children: "Anzeigen" }}
            optionFalse={{ children: "Ausblenden" }}
            onChange={setShowAdvanced}
          />
        </Label>
      </ControlCard>

      {showAdvanced && (
        <>
          <ControlGrid columns={2}>
            <ControlCard title="Druck-PID-Einstellungen">
              <Label label="Kp">
                <EditValue
                  value={state?.pid_settings.pressure.kp}
                  defaultValue={defaultState?.pid_settings.pressure.kp}
                  min={0}
                  max={100}
                  step={0.01}
                  renderValue={(v) => roundToDecimals(v, 2)}
                  onChange={setPressurePidKp}
                  title="Druck-PID KP"
                />
              </Label>
              <Label label="Ki">
                <EditValue
                  value={state?.pid_settings.pressure.ki}
                  defaultValue={defaultState?.pid_settings.pressure.ki}
                  min={0}
                  max={100}
                  step={0.01}
                  renderValue={(v) => roundToDecimals(v, 2)}
                  onChange={setPressurePidKi}
                  title="Druck-PID KI"
                />
              </Label>
              <Label label="Kd">
                <EditValue
                  value={state?.pid_settings.pressure.kd}
                  defaultValue={defaultState?.pid_settings.pressure.kd}
                  min={0}
                  max={100}
                  step={0.01}
                  renderValue={(v) => roundToDecimals(v, 2)}
                  onChange={setPressurePidKd}
                  title="Druck-PID KD"
                />
              </Label>
            </ControlCard>
            <ControlCard title="Druck-PID-Auto-Tune">
              <Alert className="mt-2 border-yellow-500/50 bg-yellow-500/10">
                <AlertTitle className="text-yellow-600">
                  Zuerst das Handbuch lesen
                </AlertTitle>
                <AlertDescription>
                  Bitte vor der Verwendung Abschnitt 2.3.1 Adaptive Druck-PID-Auto-Tune im Handbuch lesen. Dort stehen wichtige Voraussetzungen und die Schritt-fuer-Schritt-Anleitung.
                </AlertDescription>
              </Alert>
              <Label label="Druck Soll">
                <EditValue
                  value={state?.pressure_state.target_bar}
                  defaultValue={defaultState?.pressure_state.target_bar}
                  unit="bar"
                  title="Drucksollwert fuer Abstimmung"
                  description="Drucksollwert, um den der Auto-Tune schwingt"
                  min={0}
                  max={40}
                  renderValue={(v) => roundToDecimals(v, 1)}
                  onChange={setInverterTargetPressure}
                />
              </Label>
              <Label label="Abstimm-Delta">
                <EditValue
                  value={tuneDelta}
                  defaultValue={1.0}
                  unit="bar"
                  title="Abstimm-Delta"
                  description="Erlaubtes Druck-Schwingungsband um den Sollwert"
                  min={0.1}
                  max={5}
                  step={0.1}
                  renderValue={(v) => roundToDecimals(v, 1)}
                  onChange={setTuneDelta}
                />
              </Label>
              <Label label="Frequenzschritt">
                <EditValue
                  value={frequencyStepHz}
                  defaultValue={2.5}
                  title="Frequenzschritt (Hz)"
                  description="Umrichter-Frequenzabweichung um den Arbeitspunkt"
                  min={1}
                  max={5}
                  step={0.25}
                  renderValue={(v) => roundToDecimals(v, 2)}
                  onChange={setFrequencyStepHz}
                />
              </Label>
              <Label label="Aktionen">
                {state?.regulation_state.uses_rpm !== false && (
                  <p className="mb-2 text-sm text-amber-600">
                    Druckregelung muss aktiv sein, um Auto-Tune zu starten.
                  </p>
                )}
                <div className="flex gap-4">
                  <button
                    onClick={() =>
                      startPressurePidAutoTune(tuneDelta, frequencyStepHz)
                    }
                    disabled={
                      state?.regulation_state.uses_rpm !== false ||
                      state?.pid_autotune_state.state === "running"
                    }
                    className="inline-block w-fit rounded bg-blue-600 px-4 py-4 text-base text-white hover:bg-blue-700 disabled:opacity-50"
                  >
                    Auto-Tune starten
                  </button>
                  <button
                    onClick={stopPressurePidAutoTune}
                    disabled={state?.pid_autotune_state.state !== "running"}
                    className="inline-block w-fit rounded bg-red-600 px-4 py-4 text-base text-white hover:bg-red-700 disabled:opacity-50"
                  >
                    Stop
                  </button>
                </div>
              </Label>
              <Label label="Status">
                <div className="flex flex-col gap-2">
                  <span className="text-base capitalize">
                    {(state?.pid_autotune_state.state ?? "not_started").replace(
                      /_/g,
                      " ",
                    )}
                  </span>
                  <div className="h-3 w-full rounded bg-slate-200">
                    <div
                      className="h-3 rounded bg-blue-500 transition-all"
                      style={{
                        width: `${state?.pid_autotune_state.progress ?? 0}%`,
                      }}
                    />
                  </div>
                  <span className="text-muted-foreground text-sm">
                    {roundToDecimals(
                      state?.pid_autotune_state.progress ?? 0,
                      1,
                    )}
                    %
                  </span>
                </div>
              </Label>
              {state?.pid_autotune_state.result && (
                <Label label="Ergebnis">
                  <span className="text-sm">
                    Kp: {roundToDecimals(state.pid_autotune_state.result.kp, 4)}
                    &nbsp;&nbsp; Ki:{" "}
                    {roundToDecimals(state.pid_autotune_state.result.ki, 4)}
                    &nbsp;&nbsp; Kd:{" "}
                    {roundToDecimals(state.pid_autotune_state.result.kd, 4)}
                  </span>
                </Label>
              )}
            </ControlCard>
          </ControlGrid>
          <ControlGrid>
            <ControlCard title="Temperatur-PID-Einstellungen (Heizzone 3) ">
              <Label label="Kp">
                <EditValue
                  value={state?.pid_settings.temperature.front.kp}
                  defaultValue={defaultState?.pid_settings.temperature.front.kp}
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("front", "kp", v)}
                  title="Temperatur-PID KP"
                />
              </Label>
              <Label label="Ki">
                <EditValue
                  value={state?.pid_settings.temperature.front.ki}
                  defaultValue={defaultState?.pid_settings.temperature.front.ki}
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("front", "ki", v)}
                  title="Temperatur-PID KI"
                />
              </Label>
              <Label label="Kd">
                <EditValue
                  value={state?.pid_settings.temperature.front.kd}
                  defaultValue={defaultState?.pid_settings.temperature.front.kd}
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("front", "kd", v)}
                  title="Temperatur-PID KD"
                />
              </Label>
            </ControlCard>
            <ControlCard title="Temperatur-PID-Einstellungen (Heizzone 2) ">
              <Label label="Kp">
                <EditValue
                  value={state?.pid_settings.temperature.middle.kp}
                  defaultValue={
                    defaultState?.pid_settings.temperature.middle.kp
                  }
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("middle", "kp", v)}
                  title="Temperatur-PID KP"
                />
              </Label>
              <Label label="Ki">
                <EditValue
                  value={state?.pid_settings.temperature.middle.ki}
                  defaultValue={
                    defaultState?.pid_settings.temperature.middle.ki
                  }
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("middle", "ki", v)}
                  title="Temperatur-PID KI"
                />
              </Label>
              <Label label="Kd">
                <EditValue
                  value={state?.pid_settings.temperature.middle.kd}
                  defaultValue={
                    defaultState?.pid_settings.temperature.middle.kd
                  }
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("middle", "kd", v)}
                  title="Temperatur-PID KD"
                />
              </Label>
            </ControlCard>
            <ControlCard title="Temperatur-PID-Einstellungen (Heizzone 1) ">
              <Label label="Kp">
                <EditValue
                  value={state?.pid_settings.temperature.back.kp}
                  defaultValue={defaultState?.pid_settings.temperature.back.kp}
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("back", "kp", v)}
                  title="Temperatur-PID KP"
                />
              </Label>
              <Label label="Ki">
                <EditValue
                  value={state?.pid_settings.temperature.back.ki}
                  defaultValue={defaultState?.pid_settings.temperature.back.ki}
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("back", "ki", v)}
                  title="Temperatur-PID KI"
                />
              </Label>
              <Label label="Kd">
                <EditValue
                  value={state?.pid_settings.temperature.back.kd}
                  defaultValue={defaultState?.pid_settings.temperature.back.kd}
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("back", "kd", v)}
                  title="Temperatur-PID KD"
                />
              </Label>
            </ControlCard>
            <ControlCard title="Temperatur-PID-Einstellungen (Duese) ">
              <Label label="Kp">
                <EditValue
                  value={state?.pid_settings.temperature.nozzle.kp}
                  defaultValue={
                    defaultState?.pid_settings.temperature.nozzle.kp
                  }
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("nozzle", "kp", v)}
                  title="Temperatur-PID KP"
                />
              </Label>
              <Label label="Ki">
                <EditValue
                  value={state?.pid_settings.temperature.nozzle.ki}
                  defaultValue={
                    defaultState?.pid_settings.temperature.nozzle.ki
                  }
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("nozzle", "ki", v)}
                  title="Temperatur-PID KI"
                />
              </Label>
              <Label label="Kd">
                <EditValue
                  value={state?.pid_settings.temperature.nozzle.kd}
                  defaultValue={
                    defaultState?.pid_settings.temperature.nozzle.kd
                  }
                  min={0}
                  max={100}
                  step={0.001}
                  renderValue={(v) => roundToDecimals(v, 3)}
                  onChange={(v) => setTemperaturePidValue("nozzle", "kd", v)}
                  title="Temperatur-PID KD"
                />
              </Label>
            </ControlCard>
          </ControlGrid>
        </>
      )}
    </Page>
  );
}

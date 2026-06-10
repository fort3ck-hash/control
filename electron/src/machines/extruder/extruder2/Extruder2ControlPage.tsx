import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import React from "react";
import { ControlGrid } from "@/control/ControlGrid";
import {
  SelectionGroup,
  SelectionGroupBoolean,
} from "@/control/SelectionGroup";
import { HeatingZone } from "../HeatingZone";
import { Label } from "@/control/Label";
import { EditValue } from "@/control/EditValue";
import { roundToDecimals } from "@/lib/decimal";
import { useExtruder2 } from "./useExtruder";
import { TimeSeriesValueNumeric } from "@/control/TimeSeriesValue";
import { StatusBadge } from "@/control/StatusBadge";
import { MachineSelector } from "@/components/MachineConnectionDropdown";

export function Extruder2ControlPage() {
  const {
    state,
    defaultState,
    nozzleTemperature,
    nozzlePower,
    frontTemperature,
    frontPower,
    backTemperature,
    backPower,
    middleTemperature,
    middlePower,
    pressure,

    motorScrewRpm,
    motorPower,
    motorCurrent,
    totalEnergyKWh,
    combinedPower,

    setExtruderMode,
    setBackHeatingTemperature,
    setFrontHeatingTemperature,
    setMiddleHeatingTemperature,
    setNozzleHeatingTemperature,
    setInverterRegulation,
    setInverterTargetPressure,
    setInverterTargetRpm,
    setPressureControlStartTolerance,
    setPressureControlLaserReference,
    filteredLaserMachines,
    selectedLaserMachine,

    isLoading,
    isDisabled,
  } = useExtruder2();

  function isZoneReadyForExtrusion(
    temperature: number,
    targetTemperature: number,
  ) {
    // if temperature is 90% of the target temperature, then we are ready for extrusion
    return temperature >= 0.9 * targetTemperature && targetTemperature > 0.0;
  }

  const allZonesReadyForExtrude = () => {
    const frontReady = isZoneReadyForExtrusion(
      frontTemperature.current?.value ?? 0,
      state?.heating_states.front.target_temperature ?? 1,
    );
    const middleReady = isZoneReadyForExtrusion(
      middleTemperature.current?.value ?? 0,
      state?.heating_states.middle.target_temperature ?? 1,
    );
    const backReady = isZoneReadyForExtrusion(
      backTemperature.current?.value ?? 0,
      state?.heating_states.back.target_temperature ?? 1,
    );
    const nozzleReady =
      !state?.extruder_settings_state.nozzle_temperature_target_enabled ||
      isZoneReadyForExtrusion(
        nozzleTemperature.current?.value ?? 0,
        state?.heating_states.nozzle.target_temperature ?? 1,
      );

    return frontReady && middleReady && backReady && nozzleReady;
  };

  return (
    <Page>
      <ControlGrid>
        <HeatingZone
          title={"Heizzone 3"}
          heatingState={state?.heating_states.front}
          heatingTimeSeries={frontTemperature}
          heatingPower={frontPower}
          onChangeTargetTemp={setFrontHeatingTemperature}
          min={0}
          max={300}
          targetTemperatureEnabled={true}
        />
        <HeatingZone
          title={"Heizzone 2"}
          heatingState={state?.heating_states.middle}
          heatingTimeSeries={middleTemperature}
          heatingPower={middlePower}
          onChangeTargetTemp={setMiddleHeatingTemperature}
          min={0}
          max={300}
          targetTemperatureEnabled={true}
        />
        <HeatingZone
          title={"Heizzone 1"}
          heatingState={state?.heating_states.back}
          heatingTimeSeries={backTemperature}
          heatingPower={backPower}
          onChangeTargetTemp={setBackHeatingTemperature}
          min={0}
          max={300}
          targetTemperatureEnabled={true}
        />
        <HeatingZone
          title={"Düse"}
          heatingState={state?.heating_states.nozzle}
          heatingTimeSeries={nozzleTemperature}
          heatingPower={nozzlePower}
          onChangeTargetTemp={setNozzleHeatingTemperature}
          min={0}
          max={300}
          targetTemperatureEnabled={
            state?.extruder_settings_state.nozzle_temperature_target_enabled ??
            true
          }
        />
        <ControlCard className="bg-red" title="Extruderantrieb">
          {state?.inverter_status_state.overload_warning == true ? (
            <StatusBadge variant="error">
              Inverter is overloaded! Please check the extruder and reduce load
              if necessary.
            </StatusBadge>
          ) : state?.inverter_status_state.fault_occurence == true ? (
            <StatusBadge variant="error">
              Inverter encountered an error! Press the restart button in Config.
              If the issue persists, activate the extruder emergency stop to
              reset the inverter.
            </StatusBadge>
          ) : state?.inverter_status_state.running == true &&
            state.inverter_status_state.fault_occurence == false ? (
            <StatusBadge variant="success">Laeuft</StatusBadge>
          ) : null}
          {state?.inverter_status_state.running == false &&
            state.inverter_status_state.fault_occurence == false && (
              <StatusBadge variant="success">OK</StatusBadge>
            )}

          <Label label="Regelung">
            <SelectionGroupBoolean
              value={state?.regulation_state.uses_rpm}
              optionTrue={{ children: "RPM" }}
              optionFalse={{ children: "Druck" }}
              onChange={setInverterRegulation}
              disabled={isDisabled}
              loading={isLoading}
            />
          </Label>
          <div className="flex flex-row flex-wrap gap-4">
            <Label label="Ausgangsdrehzahl Soll">
              <EditValue
                value={state?.screw_state.target_rpm}
                defaultValue={defaultState?.screw_state.target_rpm}
                unit="rpm"
                title="Ausgangsdrehzahl Soll"
                min={0}
                max={44}
                renderValue={(value) => roundToDecimals(value, 0)}
                onChange={setInverterTargetRpm}
              />
            </Label>
            <Label label="Druck Soll">
              <EditValue
                value={state?.pressure_state.target_bar}
                defaultValue={defaultState?.pressure_state.target_bar}
                unit="bar"
                title="Druck Soll"
                min={0.0}
                max={270.0}
                renderValue={(value) => roundToDecimals(value, 0)}
                onChange={setInverterTargetPressure}
              />
            </Label>
            <Label label="Druck-Toleranz">
              <EditValue
                value={state?.pressure_state.pressure_start_tolerance_bar}
                defaultValue={
                  defaultState?.pressure_state.pressure_start_tolerance_bar
                }
                unit="+/- bar"
                title="Druck-Toleranz"
                min={0.1}
                max={100.0}
                renderValue={(value) => roundToDecimals(value, 1)}
                onChange={setPressureControlStartTolerance}
              />
            </Label>
          </div>
          <div className="flex flex-row flex-wrap gap-4">
            <TimeSeriesValueNumeric
              label="Rpm"
              unit="rpm"
              renderValue={(value) => roundToDecimals(value, 1)}
              timeseries={motorScrewRpm}
            />

            {state?.pressure_state?.wiring_error && (
              <StatusBadge variant="error">
                Druck kann nicht gemessen werden! Drucksensor-Verkabelung
                pruefen!
              </StatusBadge>
            )}
            <TimeSeriesValueNumeric
              label="Druck"
              unit="bar"
              renderValue={(value) => roundToDecimals(value, 1)}
              timeseries={pressure}
            />
          </div>
          <div className="flex flex-row flex-wrap gap-2">
            <StatusBadge
              variant={
                state?.pressure_state.pressure_sample_stable
                  ? "success"
                  : "error"
              }
            >
              Druckfenster{" "}
              {state?.pressure_state.pressure_sample_stable
                ? "stabil"
                : `${roundToDecimals(
                    state?.pressure_state.pressure_sample_elapsed_s ?? 0,
                    0,
                  )}/${roundToDecimals(
                    state?.pressure_state.pressure_sample_window_s ?? 20,
                    0,
                  )} s`}
            </StatusBadge>
            <StatusBadge
              variant={
                state?.pressure_state.laser_in_tolerance ? "success" : "error"
              }
            >
              Laser{" "}
              {roundToDecimals(
                state?.pressure_state.laser_tolerance_elapsed_s ?? 0,
                0,
              )}
              /
              {roundToDecimals(
                state?.pressure_state.laser_tolerance_required_s ?? 30,
                0,
              )}{" "}
              s
            </StatusBadge>
            <StatusBadge
              variant={
                state?.pressure_state.pressure_control_active
                  ? "success"
                  : state?.pressure_state.pressure_control_ready
                    ? "success"
                    : "error"
              }
            >
              Druckregelung{" "}
              {state?.pressure_state.pressure_control_active
                ? "aktiv"
                : state?.pressure_state.pressure_control_ready
                  ? "bereit"
                  : "wartet"}
            </StatusBadge>
          </div>
        </ControlCard>

        <MachineSelector
          title="Laser-Referenz"
          machines={filteredLaserMachines}
          selectedMachine={selectedLaserMachine}
          connectedMachineState={{
            machine_identification_unique:
              state?.pressure_state.laser_reference_machine ?? null,
            is_available: state?.pressure_state.laser_in_tolerance ?? false,
          }}
          setConnectedMachine={setPressureControlLaserReference}
          clearConnectedMachine={() => setPressureControlLaserReference(null)}
        />

        <ControlCard className="bg-red" title="Modus">
          <SelectionGroup<"Standby" | "Heat" | "Extrude">
            value={state?.mode_state.mode}
            orientation="vertical"
            className="grid h-full grid-cols-2 gap-2"
            options={{
              Standby: {
                children: "Bereit",
                icon: "lu:CirclePause",
                isActiveClassName: "bg-green-600",
                className: "h-full",
              },
              Heat: {
                children: "Heizen",
                icon: "lu:Flame",
                isActiveClassName: "bg-green-600",
                className: "h-full",
              },
              Extrude: {
                children: "Extrudieren",
                icon: "lu:ArrowBigLeftDash",
                isActiveClassName: "bg-green-600",
                className: "h-full",
                confirmation: allZonesReadyForExtrude()
                  ? undefined
                  : "Temperatur ist zu niedrig. Wirklich extrudieren?",
              },
            }}
            onChange={setExtruderMode}
            disabled={isDisabled}
            loading={isLoading}
          />
        </ControlCard>

        <ControlCard className="bg-blue" title="Leistungsaufnahme">
          <TimeSeriesValueNumeric
            label="Gesamtleistung"
            unit="W"
            renderValue={(value) => roundToDecimals(value, 0)}
            timeseries={combinedPower}
          />
          <TimeSeriesValueNumeric
            label="Motor Power"
            unit="W"
            renderValue={(value) => roundToDecimals(value, 0)}
            timeseries={motorPower}
          />
          <TimeSeriesValueNumeric
            label="Motor Current"
            unit="A"
            renderValue={(value) => roundToDecimals(value, 1)}
            timeseries={motorCurrent}
          />
          <TimeSeriesValueNumeric
            label="Total Energy Consumption"
            unit="kWh"
            renderValue={(value) => roundToDecimals(value, 3)}
            timeseries={totalEnergyKWh}
          />
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}

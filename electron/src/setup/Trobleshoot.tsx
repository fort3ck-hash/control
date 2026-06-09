import { Alert } from "@/components/Alert";
import { Page } from "@/components/Page";
import { SectionTitle } from "@/components/SectionTitle";
import { TouchButton } from "@/components/touch/TouchButton";
import {
  rebootHmi,
  restartBackend,
  exportLogs,
} from "@/helpers/troubleshoot_helpers";
import React, { useState } from "react";
import { toast } from "sonner";

export function TroubleshootPage() {
  const [isRebootLoading, setIsRebootLoading] = useState(false);
  const [isRestartLoading, setIsRestartLoading] = useState(false);
  const [isExportLoading, setIsExportLoading] = useState(false);

  const handleRebootHmi = async () => {
    setIsRebootLoading(true);
    try {
      const result = await rebootHmi();
      if (result.success) {
        toast.success("HMI-Panel-Neustart gestartet");
      } else {
        toast.error(`HMI-Neustart fehlgeschlagen: ${result.error}`);
      }
    } catch (error) {
      toast.error(`HMI-Neustart fehlgeschlagen: ${error}`);
    } finally {
      setIsRebootLoading(false);
    }
  };

  const handleRestartBackend = async () => {
    setIsRestartLoading(true);
    try {
      const result = await restartBackend();
      if (result.success) {
        toast.success("Backend-Service-Neustart gestartet");
      } else {
        toast.error(`Backend-Neustart fehlgeschlagen: ${result.error}`);
      }
    } catch (error) {
      toast.error(`Backend-Neustart fehlgeschlagen: ${error}`);
    } finally {
      setIsRestartLoading(false);
    }
  };

  const handleExportLogs = async () => {
    setIsExportLoading(true);
    try {
      const result = await exportLogs();
      if (result.success) {
        toast.success("Log-Export gestartet");
      } else {
        toast.error(`Log-Export fehlgeschlagen: ${result.error}`);
      }
    } catch (error) {
      toast.error(`Log-Export fehlgeschlagen: ${error}`);
    } finally {
      setIsExportLoading(false);
    }
  };

  return (
    <Page>
      <SectionTitle title="Systemfehlersuche" />

      <Alert title="Hinweis zu Fehlersuche-Aktionen" variant="warning">
        Diese Aktionen unterbrechen den Anlagenbetrieb kurzzeitig. Der HMI-Neustart startet das komplette Panel neu, waehrend der Backend-Neustart nur den Steuerungsdienst neu startet. Im Produktionsbetrieb vorsichtig verwenden.
      </Alert>

      <div className="flex gap-4">
        <TouchButton
          variant="destructive"
          icon="lu:Power"
          isLoading={isRebootLoading}
          onClick={handleRebootHmi}
          className="w-max"
        >
          Reboot HMI Panel
        </TouchButton>

        <TouchButton
          variant="outline"
          icon="lu:RotateCcw"
          isLoading={isRestartLoading}
          onClick={handleRestartBackend}
          className="w-max"
        >
          Restart Backend Process
        </TouchButton>

        <TouchButton
          variant="outline"
          icon="lu:FileDown"
          isLoading={isExportLoading}
          onClick={exportLogs}
          className="w-max"
        >
          Export Backend Service Logs
        </TouchButton>
      </div>
    </Page>
  );
}

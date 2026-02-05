import { useCallback, useEffect, useMemo, useState } from "react";
import type { ProviderListItem } from "../../../services/refact";
import { ConfiguredProvidersViewProps } from "./ConfiguredProvidersView";

export function useGetConfiguredProvidersView({
  configuredProviders,
  handleSetCurrentProvider,
}: {
  configuredProviders: ConfiguredProvidersViewProps["configuredProviders"];
  handleSetCurrentProvider: ConfiguredProvidersViewProps["handleSetCurrentProvider"];
}) {
  const notConfiguredProviders = useMemo(() => {
    return configuredProviders.filter((p) => !p.enabled && !p.readonly);
  }, [configuredProviders]);

  const [potentialCurrentProvider, setPotentialCurrentProvider] = useState<
    ProviderListItem | undefined
  >(notConfiguredProviders[0] || undefined);

  const sortedConfiguredProviders = useMemo(() => {
    return [...configuredProviders].sort((a, b) => {
      const getPriority = (provider: { name: string }) => {
        if (
          provider.name === "refact" ||
          provider.name === "refact_self_hosted"
        )
          return 0;
        if (provider.name === "custom") return 2;
        return 1;
      };

      const priorityA = getPriority(a);
      const priorityB = getPriority(b);

      if (priorityA !== priorityB) {
        return priorityA - priorityB;
      }

      return a.name.localeCompare(b.name);
    });
  }, [configuredProviders]);

  const handlePotentialCurrentProvider = useCallback(
    (value: string) => {
      const provider = configuredProviders.find((p) => p.name === value);
      if (provider) {
        setPotentialCurrentProvider(provider);
      }
    },
    [configuredProviders],
  );

  const handleAddNewProvider = useCallback(() => {
    if (!potentialCurrentProvider) return;
    handleSetCurrentProvider(potentialCurrentProvider);
  }, [handleSetCurrentProvider, potentialCurrentProvider]);

  useEffect(() => {
    if (notConfiguredProviders.length > 0) {
      setPotentialCurrentProvider(notConfiguredProviders[0]);
    }
  }, [notConfiguredProviders]);

  return {
    handlePotentialCurrentProvider,
    handleAddNewProvider,
    sortedConfiguredProviders,
    notConfiguredProviderTemplates: notConfiguredProviders,
    potentialCurrentProvider,
  };
}

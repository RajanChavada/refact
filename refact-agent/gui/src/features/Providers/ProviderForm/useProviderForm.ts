import isEqual from "lodash.isequal";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { ProviderDetailResponse } from "../../../services/refact";
import {
  useGetConfiguredProvidersQuery,
  useGetProviderQuery,
} from "../../../hooks/useProvidersQuery";

export type ProviderFormValues = {
  enabled: boolean;
  readonly: boolean;
  [key: string]: unknown;
};

export function useProviderForm({ providerName }: { providerName: string }) {
  const { data: providerDetail, isSuccess: isProviderLoadedSuccessfully } =
    useGetProviderQuery({
      providerName: providerName,
    });
  const { data: configuredProviders } = useGetConfiguredProvidersQuery();

  const [formValues, setFormValues] = useState<ProviderFormValues | null>(null);
  const [areShowingExtraFields, setAreShowingExtraFields] = useState(false);

  // Convert provider detail to form values
  useEffect(() => {
    if (providerDetail) {
      setFormValues({
        enabled: providerDetail.enabled,
        readonly: providerDetail.readonly,
        ...providerDetail.settings,
      });
    }
  }, [providerDetail]);

  const shouldSaveButtonBeDisabled = useMemo(() => {
    if (!providerDetail || !formValues) return true;

    const isProviderConfigured = configuredProviders?.providers.some(
      (p) => p.name === providerName && p.enabled,
    );
    if (!isProviderConfigured) return false;

    const originalFormValues = {
      enabled: providerDetail.enabled,
      readonly: providerDetail.readonly,
      ...providerDetail.settings,
    };

    return providerDetail.readonly || isEqual(formValues, originalFormValues);
  }, [configuredProviders, providerDetail, formValues, providerName]);

  const handleFormValuesChange = useCallback(
    (updatedProviderData: ProviderFormValues) => {
      setFormValues(updatedProviderData);
    },
    [],
  );

  // Convert ProviderDetailResponse to legacy format for backward compatibility
  const detailedProvider: ProviderDetailResponse | undefined = providerDetail;

  return {
    formValues,
    setFormValues,
    areShowingExtraFields,
    setAreShowingExtraFields,
    shouldSaveButtonBeDisabled,
    handleFormValuesChange,
    configuredProviders,
    detailedProvider,
    isProviderLoadedSuccessfully,
  };
}

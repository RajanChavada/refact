import { useMemo, useState, type FC } from "react";
import {
  Badge,
  Button,
  Callout,
  Flex,
  Heading,
  Separator,
  Text,
} from "@radix-ui/themes";
import { PlusIcon, InfoCircledIcon } from "@radix-ui/react-icons";

import type { ProviderListItem } from "../../../../services/refact";
import { useGetAvailableModelsQuery } from "../../../../services/refact";

import { Spinner } from "../../../../components/Spinner";
import { AvailableModelCard } from "./AvailableModelCard";
import { AddCustomModelModal } from "./AddCustomModelModal";

export type ProviderModelsListProps = {
  provider: ProviderListItem;
};

export const ProviderModelsList: FC<ProviderModelsListProps> = ({
  provider,
}) => {
  const {
    data: modelsData,
    isSuccess,
    isLoading,
    isError,
    error,
  } = useGetAvailableModelsQuery({ providerName: provider.name });

  const [isAddModalOpen, setIsAddModalOpen] = useState(false);

  // Separate enabled and disabled models
  const { enabledModels, disabledModels } = useMemo(() => {
    if (!modelsData?.models) return { enabledModels: [], disabledModels: [] };

    const enabled = modelsData.models.filter((m) => m.enabled);
    const disabled = modelsData.models.filter((m) => !m.enabled);

    return { enabledModels: enabled, disabledModels: disabled };
  }, [modelsData?.models]);

  if (isLoading) return <Spinner spinning />;

  if (isError) {
    const err = error as { status?: unknown; data?: { detail?: unknown } } | undefined;
    const errorMessage = err?.status
      ? `${String(err.status)}: ${err.data?.detail ? String(err.data.detail) : "Unknown error"}`
      : "Failed to load models";

    return (
      <Callout.Root color="red">
        <Callout.Icon>
          <InfoCircledIcon />
        </Callout.Icon>
        <Callout.Text>Failed to load models: {errorMessage}</Callout.Text>
      </Callout.Root>
    );
  }

  if (!isSuccess) {
    return (
      <Callout.Root color="orange">
        <Callout.Icon>
          <InfoCircledIcon />
        </Callout.Icon>
        <Callout.Text>
          No model data available. Make sure the provider is properly configured.
        </Callout.Text>
      </Callout.Root>
    );
  }

  const totalModels = modelsData.models.length;
  const enabledCount = enabledModels.length;

  return (
    <Flex direction="column" gap="3" mt="4">
      <Separator size="4" />

      <Flex align="center" justify="between">
        <Flex align="center" gap="2">
          <Heading as="h3" size="3">
            Available Models
          </Heading>
          <Badge size="1" color="gray">
            {enabledCount}/{totalModels} enabled
          </Badge>
        </Flex>

        {!provider.readonly && (
          <Button
            size="1"
            variant="soft"
            onClick={() => setIsAddModalOpen(true)}
          >
            <PlusIcon /> Add Custom Model
          </Button>
        )}
      </Flex>

      {modelsData.error && (
        <Callout.Root color="orange" size="1">
          <Callout.Icon>
            <InfoCircledIcon />
          </Callout.Icon>
          <Callout.Text size="1">{modelsData.error}</Callout.Text>
        </Callout.Root>
      )}

      {totalModels === 0 ? (
        <Flex direction="column" align="center" gap="2" py="4">
          <Text as="span" size="2" color="gray">
            No models available for this provider.
          </Text>
          {!provider.readonly && (
            <Text as="span" size="1" color="gray">
              Click &quot;Add Custom Model&quot; to define your own.
            </Text>
          )}
        </Flex>
      ) : (
        <Flex direction="column" gap="2">
          {/* Enabled models first */}
          {enabledModels.length > 0 && (
            <>
              <Text as="span" size="1" color="gray" weight="medium">
                Enabled ({enabledModels.length})
              </Text>
              {enabledModels.map((model) => (
                <AvailableModelCard
                  key={model.id}
                  model={model}
                  providerName={provider.name}
                  isReadonlyProvider={provider.readonly}
                />
              ))}
            </>
          )}

          {/* Disabled models */}
          {disabledModels.length > 0 && (
            <>
              <Text as="span" size="1" color="gray" weight="medium" mt="2">
                Available ({disabledModels.length})
              </Text>
              {disabledModels.map((model) => (
                <AvailableModelCard
                  key={model.id}
                  model={model}
                  providerName={provider.name}
                  isReadonlyProvider={provider.readonly}
                />
              ))}
            </>
          )}
        </Flex>
      )}

      <AddCustomModelModal
        providerName={provider.name}
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
      />
    </Flex>
  );
};

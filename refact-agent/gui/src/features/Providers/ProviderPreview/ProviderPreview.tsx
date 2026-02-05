import React from "react";
import { Flex, Heading } from "@radix-ui/themes";

import { ProviderForm } from "../ProviderForm";

import { useProviderPreview } from "./useProviderPreview";
import { getProviderName } from "../getProviderName";

import type { ProviderListItem } from "../../../services/refact";
import { DeletePopover } from "../../../components/DeletePopover";

export type ProviderPreviewProps = {
  configuredProviders: ProviderListItem[];
  currentProvider: ProviderListItem;
  handleSetCurrentProvider: (provider: ProviderListItem | null) => void;
};

export const ProviderPreview: React.FC<ProviderPreviewProps> = ({
  configuredProviders,
  currentProvider,
  handleSetCurrentProvider,
}) => {
  const {
    handleDiscardChanges,
    handleSaveChanges,
    handleDeleteProvider,
    isDeletingProvider,
    isSavingProvider,
  } = useProviderPreview(handleSetCurrentProvider);

  return (
    <Flex direction="column" align="start" height="100%">
      <Flex justify="between" align="center" width="100%" mb="4">
        <Heading as="h2" size="3">
          {getProviderName(currentProvider)} Configuration
        </Heading>
        <DeletePopover
          itemName={getProviderName(currentProvider)}
          isDisabled={currentProvider.readonly}
          isDeleting={isDeletingProvider}
          deleteBy={currentProvider.name}
          handleDelete={(providerName: string) =>
            void handleDeleteProvider(providerName)
          }
        />
      </Flex>
      <ProviderForm
        currentProvider={currentProvider}
        handleSaveChanges={(updatedProviderData) =>
          void handleSaveChanges(updatedProviderData, currentProvider.name)
        }
        isSaving={isSavingProvider}
        isProviderConfigured={configuredProviders.some(
          (p) => p.name === currentProvider.name && p.enabled,
        )}
        handleDiscardChanges={handleDiscardChanges}
      />
    </Flex>
  );
};

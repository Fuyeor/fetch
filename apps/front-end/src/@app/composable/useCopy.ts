// @app/composable/useCopy.ts
import { useLocale } from '@fuyeor/locale';
import { useToast } from '@fuyeor/interactify';

export function useCopy() {
  const { t } = useLocale();
  const { showToast } = useToast();

  async function copyText(text: string) {
    await window.navigator.clipboard.writeText(text);
    showToast(t('copy.success'), { type: 'success' });
  }

  return { copyText };
}

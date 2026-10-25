<script lang="ts">
  import { iconByName } from '../../icons/catalog'
  import { cardTemplates, type CardTemplateId } from '../../utils/cardTemplates'

  type Props = {
    value?: string
    onselect: (id: CardTemplateId) => void
  }

  let { value = 'classic', onselect }: Props = $props()
</script>

<div class="grid grid-cols-2 gap-2">
  {#each cardTemplates as template}
    {@const icon = template.icon ? iconByName.get(template.icon) : undefined}
    <button
      type="button"
      class="rounded-xl border p-1.5 text-left transition-colors {value === template.id
        ? 'border-forest-700 bg-forest-50/70 ring-1 ring-forest-700'
        : 'border-line bg-white hover:border-gold-300 hover:bg-gold-50/30'}"
      data-card-template-option={template.id}
      title={template.description}
      onclick={() => onselect(template.id)}
    >
      <div
        class="relative h-14 overflow-hidden px-2 py-1.5"
        style:background={template.fill}
        style:border-radius={`${Math.max(template.radius, 1) * 3}px`}
        style:border-bottom={template.id === 'minimal' ? `2px solid ${template.accent}` : 'none'}
        style:border-left={template.id === 'classic' ? `3px solid ${template.accent}` : 'none'}
      >
        {#if template.layout === 'right'}
          <span class="absolute top-1.5 right-1.5 flex h-5 w-5 items-center justify-center rounded-full" style:background={template.iconBg} style:color={template.iconColor}>
            {#if icon}<svg class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{@html icon.svg}</svg>{/if}
          </span>
          <strong class="block text-sm leading-none" style:color={template.color}>12</strong>
          <span class="absolute bottom-1.5 left-2 text-[7px]" style:color={template.color}>Label</span>
        {:else if template.layout === 'left'}
          <div class="flex h-full items-center gap-1.5">
            <span class="flex h-5 w-5 shrink-0 items-center justify-center" style:color={template.iconColor}>
              {#if icon}<svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{@html icon.svg}</svg>{/if}
            </span>
            <span class="min-w-0">
              <strong class="block text-sm leading-none" style:color={template.color}>12</strong>
              <span class="block truncate text-[7px]" style:color={template.color}>Label</span>
            </span>
          </div>
        {:else if template.layout === 'top'}
          <div class="flex h-full flex-col items-center justify-center text-center" style:color={template.color}>
            <span class="flex h-4 w-4 items-center justify-center rounded-full" style:background={template.iconBg} style:color={template.iconColor}>
              {#if icon}<svg class="h-2.5 w-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{@html icon.svg}</svg>{/if}
            </span>
            <strong class="mt-0.5 text-sm leading-none">12</strong>
            <span class="text-[7px]">Label</span>
          </div>
        {:else if template.layout === 'panel'}
          <div class="flex h-full overflow-hidden rounded">
            <span class="flex w-[28%] items-center justify-center" style:background={template.accent} style:color={template.iconColor}>
              {#if icon}<svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{@html icon.svg}</svg>{/if}
            </span>
            <span class="flex min-w-0 flex-1 flex-col justify-center pl-1.5" style:color={template.color}>
              <strong class="text-sm leading-none">12</strong>
              <span class="truncate text-[7px]">Label</span>
            </span>
          </div>
        {:else}
          <strong class="block text-sm leading-none" style:color={template.color}>12</strong>
          <span class="mt-3 block text-[7px]" style:color={template.color}>Label</span>
        {/if}
      </div>
      <span class="mt-1 block text-[10px] font-semibold text-ink">{template.label}</span>
    </button>
  {/each}
</div>

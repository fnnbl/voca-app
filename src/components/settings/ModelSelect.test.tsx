import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi } from 'vitest'
import { ModelSelect } from './ModelSelect'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}))

const base = {
  onChange: () => {},
  loading: false,
  failed: false,
}

describe('ModelSelect', () => {
  it('shows the static list until a live list arrives', () => {
    render(<ModelSelect {...base} value="a" liveModels={null} fallbackModels={['a', 'b']} />)
    expect(screen.getByRole('option', { name: 'b' })).toBeInTheDocument()
  })

  it('prefers the live list over the static one', () => {
    render(<ModelSelect {...base} value="x" liveModels={['x', 'y']} fallbackModels={['a']} />)
    expect(screen.getByRole('option', { name: 'y' })).toBeInTheDocument()
    expect(screen.queryByRole('option', { name: 'a' })).not.toBeInTheDocument()
  })

  it('flags a saved model the live list no longer contains', () => {
    render(<ModelSelect {...base} value="gpt-4-turbo" liveModels={['gpt-6-luna']} fallbackModels={[]} />)
    expect(
      screen.getByRole('option', { name: 'gpt-4-turbo (settings.models.unavailable)' }),
    ).toBeInTheDocument()
    expect(screen.getByText('settings.models.unavailableHint')).toBeInTheDocument()
  })

  it('does not flag an unlisted model against the static fallback', () => {
    render(<ModelSelect {...base} value="my-model" liveModels={null} fallbackModels={['a']} />)
    expect(screen.getByRole('option', { name: 'my-model' })).toBeInTheDocument()
    expect(screen.queryByText('settings.models.unavailableHint')).not.toBeInTheDocument()
  })

  it('switches to free-text entry for a custom model', async () => {
    const onChange = vi.fn()
    render(<ModelSelect {...base} onChange={onChange} value="a" liveModels={null} fallbackModels={['a']} />)
    await userEvent.selectOptions(screen.getByRole('combobox'), 'settings.models.custom')
    expect(onChange).not.toHaveBeenCalled()
    await userEvent.type(screen.getByPlaceholderText('settings.models.customPlaceholder'), 'z')
    expect(onChange).toHaveBeenLastCalledWith('az')
  })

  it('starts in free-text mode when no models are listed', () => {
    render(<ModelSelect {...base} value="" liveModels={null} fallbackModels={[]} />)
    expect(screen.getByPlaceholderText('settings.models.customPlaceholder')).toBeInTheDocument()
  })
})

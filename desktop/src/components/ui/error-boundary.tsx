import { Component, type ReactNode } from "react";
import { I18nContext, initialLocale, translate } from "../../lib/i18n";
import { ErrorState } from "./shared";
export class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: string | null }
> {
  state: { error: string | null } = { error: null };
  static getDerivedStateFromError(error: Error) {
    return { error: error.message };
  }
  render() {
    if (this.state.error) {
      const locale = initialLocale();
      return (
        <I18nContext.Provider value={locale}>
          <div className="page">
            <ErrorState
              error={translate(
                "The interface encountered a problem. Saved records remain in your local books. {error}",
                locale,
                { error: this.state.error },
              )}
              retry={() => this.setState({ error: null })}
            />
          </div>
        </I18nContext.Provider>
      );
    }
    return this.props.children;
  }
}

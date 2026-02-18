import { WizardProvider } from "./context/WizardContext";
import { AssistantProvider } from "./context/AssistantContext";
import { useAssistantContext } from "./context/AssistantContext";
import { WizardRouter } from "./components/WizardRouter";
import { AssistantPanel } from "./components/assistant";

function App() {
  return (
    <WizardProvider>
      <AssistantProvider>
        <AppLayout />
      </AssistantProvider>
    </WizardProvider>
  );
}

function AppLayout() {
  const { isOpen } = useAssistantContext();
  
  return (
    <>
      <div className={`app-main-content ${isOpen ? 'assistant-open' : ''}`}>
        <WizardRouter />
      </div>
      <AssistantPanel />
    </>
  );
}

export default App;

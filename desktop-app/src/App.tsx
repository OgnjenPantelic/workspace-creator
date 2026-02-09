import { WizardProvider } from "./context/WizardContext";
import { WizardRouter } from "./components/WizardRouter";

function App() {
  return (
    <WizardProvider>
      <WizardRouter />
    </WizardProvider>
  );
}

export default App;

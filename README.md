# Project BioMesh: Duckweed & Cultivation Toolset

A consolidated, tested Python package of calculators and simulators for home/hobby-scale hydroponic growing, specifically optimized for **VALAGRO MASTER 15-5-30+TE** and biological **duckweed/watermeal (Lemna/Wolffia)** cultivation.

---

## Workspace Directory Structure

All core components and scripts are organized at the root level of the workspace for standard modular integration:

```text
duckweed_tools/
├── composition.py            # Fertilizer inputs (Valagro Master + Urea) and elemental conversions
├── dosing.py                 # Forward/reverse concentration calculators (ppm <-> grams)
├── ec_estimator.py           # EC approximations (500 and 700 scale)
├── stock_solution.py         # Concentrated stock preparation & dilution workflows
├── scheduler.py              # Stage-by-stage crop scheduling & totals bookkeeping
├── duckweed_simulator.py     # Vessel water quality modeling, ranges & weekly program simulator
│
├── tests/                    # Automated Unit Tests
│   ├── test_calculators.py   # 23 tests verifying composition, dosing, EC, stock, and scheduling math
│   └── test_simulator.py     # 22 tests verifying ranges, vessel pools, and biological simulations
│
├── cultivation_log.json      # Structured empirical database tracking the 12-day trial
├── cultivation_log.md        # Automatically generated human-readable log book
├── manage_log.py             # CLI utility for logging new entries and exporting log books
│
├── requirements.txt          # Python project test dependencies
├── Duckweed_Research_Book_EN.pdf                               # Reference: 256 compiled findings
└── Project BioMesh- Technical Blueprint and Operational V2.pdf  # Reference: BioMesh framework V2
```

---

## Getting Started

### 1. Initialize Virtual Environment
Set up the local virtual environment and install the required test runners:

```bash
# Create the virtual environment
python -m venv .venv

# Activate the virtual environment
# On Windows (PowerShell):
.venv\Scripts\Activate.ps1
# On macOS/Linux:
source .venv/bin/activate

# Install dependencies
pip install -r requirements.txt
```

### 2. Run the Unified Test Suite
Run the full suite of 45 passing unit tests:

```bash
pytest tests/ -v
```

---

## Log Management Tooling

To keep empirical research logs structured and updatable, we utilize a unified log manager CLI (`manage_log.py`) which acts as the interface to the underlying `cultivation_log.json` database.

### Command Usage:
* **View Summary**: Prints a brief console summary of the registered cultivation days.
  ```bash
  python manage_log.py view
  ```
* **Log New Day**: Launches an interactive prompt to log parameters, operations, observations, and additives. Input values are automatically validated.
  ```bash
  python manage_log.py append
  ```
* **Export Markdown**: Renders the complete database into a clean, markdown-styled log file (`cultivation_log.md`) complete with alerts and warnings.
  ```bash
  python manage_log.py export
  ```

---

## Module Roles and Integrations

* **`composition.py`**: Holds chemical specs for Valagro Master 15-5-30+TE and Urea ($CH_4N_2O$). Standardizes textbook multipliers converting oxide forms to plant-available elements ($P_2O_5 \rightarrow P$, $K_2O \rightarrow K$, $MgO \rightarrow Mg$).
* **`dosing.py`**: Calculates the precise mass of inputs needed to hit target elemental concentrations.
* **`ec_estimator.py`**: Approximates Electrical Conductivity (EC) based on input mass, accommodating Hanna/700 and NaCl/500 scales.
* **`stock_solution.py`**: Calculates stock bottle concentrations and dilution ratios.
* **`scheduler.py`**: Generates growth-stage schedulers, compiling water and nutrient mass tracking across growth weeks.
* **`duckweed_simulator.py`**: Integrates physiological limits from the *Duckweed Research Book* (Chapter 4) for $NO_3\text{-N}$, $NH_4\text{-N}$, $P$, $K$, and $Mg$. Simulates repeat weekly schedules to alert growers of salt saturation and toxicity.

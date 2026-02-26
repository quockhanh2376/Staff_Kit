---
name: backend-refactoring
description: Use when refactoring FastAPI backend architecture for vn-stock-portfolio. Enforce modular design, separation of concerns, router decomposition, service layer patterns, and maintainability.
version: 1.0.0
scope: workspace
tags: [backend, fastapi, refactoring, architecture, python, sqlalchemy]
---

# Backend Refactoring Guidelines (VN Stock Portfolio)

## Goal
Maintain clean, modular, and maintainable FastAPI backend architecture following separation of concerns and SOLID principles.

## When to use
- Decomposing large routers into sub-routers
- Refactoring monolithic services
- Organizing business logic layers
- Improving code maintainability
- Reducing technical debt

## Tech Stack Context
- **Framework**: FastAPI 0.128.0 + Uvicorn
- **Database**: PostgreSQL + SQLAlchemy 2.0.45 + Alembic
- **Cache**: Redis 7.1.0
- **Background Jobs**: RQ + APScheduler
- **Data Sources**: vnstock3, vnai (Vietnamese stock market)
- **Python**: 3.12+

## Architecture Principles

### 1. Directory Structure
```
backend/
├── main.py              # Entry point (minimal, delegates to app_factory)
├── app_factory.py       # FastAPI app creation
├── middleware.py        # Middleware configuration
├── core/                # Core utilities (db, logger, config, etc.)
├── models/              # SQLAlchemy ORM models
├── schemas/             # Pydantic request/response models
├── routers/             # API route handlers (thin layer)
│   ├── auth/           # Auth sub-routers
│   ├── admin_routes/   # Admin sub-routers
│   └── *.py            # Feature routers
├── services/            # Business logic layer (thick layer)
│   ├── admin_ops/      # Admin operations
│   ├── market/         # Market data services
│   └── *.py            # Feature services
├── adapters/            # External API adapters (vnstock, etc.)
├── tasks/               # Background job definitions
└── scripts/             # Operational scripts
    └── debug/          # Categorized debug scripts
```

### 2. Layered Architecture

**Layer 1: Routers (Thin)**
- Handle HTTP concerns (request/response)
- Validate input via Pydantic schemas
- Call service layer
- Return standardized responses
- **Keep routers under 200 lines**

**Layer 2: Services (Thick)**
- Contain business logic
- Orchestrate database operations
- Handle external API calls
- Implement domain rules
- **Single Responsibility Principle**

**Layer 3: Models & Schemas**
- SQLAlchemy models for database
- Pydantic schemas for API contracts
- Clear separation between DB and API models

**Layer 4: Adapters**
- Wrap external APIs (vnstock, vnai)
- Provide consistent interface
- Handle API-specific quirks

## Refactoring Patterns

### Pattern 1: Router Decomposition

**Problem**: Router file exceeds 200 lines or handles multiple sub-resources.

**Solution**: Create sub-router directory structure.

**Example**:
```python
# Before: routers/admin.py (300+ lines)
@router.get("/admin/users")
@router.post("/admin/users")
@router.get("/admin/system-health")
@router.post("/admin/cache-clear")

# After: routers/admin.py (orchestrator)
from routers.admin_routes import users, system

router = APIRouter(prefix="/admin", tags=["admin"])
router.include_router(users.router)
router.include_router(system.router)

# routers/admin_routes/users.py
router = APIRouter(prefix="/users", tags=["admin-users"])

@router.get("")
async def list_users(db: Session = Depends(get_db)):
    return await user_service.list_users(db)
```

**Checklist**:
- [ ] Create `routers/{feature}_routes/` directory
- [ ] Split by sub-resource (users, settings, etc.)
- [ ] Each sub-router < 150 lines
- [ ] Update main router to include sub-routers
- [ ] Preserve API paths (no breaking changes)

### Pattern 2: Service Layer Extraction

**Problem**: Business logic mixed in router handlers.

**Solution**: Extract to dedicated service modules.

**Example**:
```python
# Before: routers/portfolio.py (bad)
@router.get("/portfolio")
async def get_portfolio(db: Session = Depends(get_db)):
    holdings = db.query(TickerHolding).filter(...).all()
    # 50 lines of calculation logic here
    return {"data": result}

# After: routers/portfolio.py (good)
@router.get("/portfolio")
async def get_portfolio(db: Session = Depends(get_db)):
    result = await portfolio_service.get_portfolio_summary(db)
    return {"ok": True, "data": result}

# services/portfolio_service.py
async def get_portfolio_summary(db: Session) -> PortfolioSummary:
    holdings = db.query(TickerHolding).filter(...).all()
    # Business logic here
    return calculate_portfolio_metrics(holdings)
```

**Checklist**:
- [ ] Create service module in `services/`
- [ ] Move business logic to service functions
- [ ] Keep database queries in service layer
- [ ] Router only handles HTTP concerns
- [ ] Add type hints for service functions

### Pattern 3: Service Decomposition

**Problem**: Service file exceeds 300 lines or handles multiple concerns.

**Solution**: Split into sub-modules by domain.

**Example**:
```python
# Before: services/market_service.py (500+ lines)
def get_market_summary(): ...
def get_ticker_price(): ...
def calculate_indicators(): ...
def fetch_historical_data(): ...

# After: services/market/
# services/market/__init__.py
from .summary import get_market_summary
from .pricing import get_ticker_price
from .indicators import calculate_indicators
from .data_fetching import fetch_historical_data

# services/market/summary.py
def get_market_summary(): ...

# services/market/pricing.py
def get_ticker_price(): ...
```

**Checklist**:
- [ ] Create `services/{domain}/` directory
- [ ] Split by functional area
- [ ] Each module < 250 lines
- [ ] Update imports in routers
- [ ] Maintain backward compatibility

### Pattern 4: Adapter Consolidation

**Problem**: Multiple adapters for similar data sources (e.g., vnstock_adapter + vnstock_smart_adapter).

**Solution**: Merge into single adapter with strategy pattern.

**Example**:
```python
# Before: Two separate adapters
# adapters/vnstock_adapter.py
# adapters/vnstock_smart_adapter.py

# After: Single unified adapter
# adapters/vnstock_adapter.py
class VnstockAdapter:
    def __init__(self, strategy: str = "default"):
        self.strategy = strategy
    
    def fetch_price(self, ticker: str):
        if self.strategy == "smart":
            return self._fetch_smart(ticker)
        return self._fetch_default(ticker)
```

**Checklist**:
- [ ] Identify common interface
- [ ] Create unified adapter class
- [ ] Use strategy/factory pattern for variants
- [ ] Update all usages
- [ ] Remove duplicate code

### Pattern 5: Main.py Decomposition

**Problem**: main.py contains app initialization, middleware, and startup logic.

**Solution**: Already implemented via app_factory.py pattern.

**Current Structure** (Good ✅):
```python
# main.py - Entry point only
from app_factory import create_app
app = create_app()

# app_factory.py - App creation
def create_app() -> FastAPI:
    app = FastAPI(lifespan=lifespan)
    configure_middleware(app)
    register_exception_handlers(app)
    register_routers(app)
    return app

# middleware.py - Middleware config
def configure_middleware(app: FastAPI):
    app.add_middleware(CORSMiddleware, ...)
```

## Code Quality Standards

### 1. File Size Limits
- **Routers**: < 200 lines (split if larger)
- **Services**: < 300 lines (split if larger)
- **Models**: < 400 lines (split if larger)
- **Utilities**: < 250 lines

### 2. Function Complexity
- Max 50 lines per function
- Max 4 levels of nesting
- Single Responsibility Principle

### 3. Naming Conventions
- **Routers**: `{feature}.py` or `{feature}_routes/`
- **Services**: `{feature}_service.py` or `{domain}/`
- **Models**: `{entity}.py` (singular)
- **Schemas**: `{entity}.py` or `{feature}.py`

### 4. Import Organization
```python
# Standard library
from __future__ import annotations
import sys
from datetime import datetime

# Third-party
from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

# Local core
from core.db import get_db
from core.logger import logger

# Local domain
import models
import schemas
from services import portfolio_service
```

## Refactoring Workflow

### Step 1: Analyze
- Identify large files (> size limits)
- Find duplicated logic
- Locate mixed concerns

### Step 2: Plan
- Define split boundaries
- Design new structure
- Document breaking changes

### Step 3: Execute
- Create new modules
- Move code incrementally
- Update imports
- Preserve functionality

### Step 4: Verify
- Run existing tests
- Check API contracts unchanged
- Verify no regressions
- Update documentation

### Step 5: Clean Up
- Remove old files
- Update references
- Commit with clear message

## Common Refactoring Tasks

### Task: Split Dense Router
1. Create `routers/{feature}_routes/` directory
2. Group endpoints by sub-resource
3. Create sub-router files
4. Update main router to include sub-routers
5. Test all endpoints still work

### Task: Extract Service Logic
1. Create `services/{feature}_service.py`
2. Move business logic from router
3. Add type hints and docstrings
4. Update router to call service
5. Test functionality preserved

### Task: Merge Duplicate Adapters
1. Identify common interface
2. Create unified adapter
3. Implement strategy pattern
4. Update all usages
5. Remove old adapters

### Task: Organize Debug Scripts
1. Categorize by purpose (already done ✅)
2. Archive obsolete scripts
3. Document each category
4. Create READMEs

## Anti-Patterns to Avoid

❌ **Don't**: Put business logic in routers
✅ **Do**: Keep routers thin, delegate to services

❌ **Don't**: Mix database queries with HTTP handling
✅ **Do**: Separate concerns into layers

❌ **Don't**: Create circular dependencies
✅ **Do**: Follow dependency direction (router → service → model)

❌ **Don't**: Use global state
✅ **Do**: Use dependency injection

❌ **Don't**: Ignore file size limits
✅ **Do**: Split proactively when approaching limits

## Quick Checklist

When refactoring backend code:
- [ ] Routers are thin (< 200 lines)
- [ ] Business logic in services
- [ ] Clear separation of concerns
- [ ] No circular dependencies
- [ ] Type hints on public functions
- [ ] Docstrings on complex logic
- [ ] Tests still pass
- [ ] API contracts unchanged
- [ ] Imports organized
- [ ] No duplicate code

## References

- [FastAPI Best Practices](https://fastapi.tiangolo.com/tutorial/)
- [SQLAlchemy 2.0 Patterns](https://docs.sqlalchemy.org/en/20/)
- Project skills: `api-contract`, `db-sqlalchemy-migrations`

# UML Model Layer Analysis

> Produced 2026-06-23 — based on exhaustive inspection of 79 source files in `umbrello/umlmodel/`.

---

## Table of Contents

1. [Full Class Hierarchy](#1-full-class-hierarchy)
2. [Class-by-Class Role Descriptions](#2-class-by-class-role-descriptions)
3. [UML Entity Taxonomy](#3-uml-entity-taxonomy)
4. [Relationship Types and Modelling](#4-relationship-types-and-modelling)
5. [Ownership and Containment Model](#5-ownership-and-containment-model)
6. [XMI Serialization Patterns](#6-xmi-serialization-patterns)
7. [Technical Debt Inventory](#7-technical-debt-inventory)
8. [Rust Recommendations](#8-rust-recommendations)
9. [Proposed Rust Type Hierarchy](#9-proposed-rust-type-hierarchy)
10. [Migration Considerations](#10-migration-considerations)

---

## 1. Full Class Hierarchy

Legend: `C` = concrete, `A` = abstract, `L` = leaf (instantiable in practice).

```
QObject  (Qt root)
 │
 └── UMLObject  (A)  — root of all UML model elements
      │                 Fields: m_nId, m_name, m_BaseType, m_visibility,
      │                         m_bAbstract, m_bStatic, m_Doc, m_pStereotype,
      │                         m_pSecondary, m_SecondaryId, m_SecondaryFallback,
      │                         m_TaggedValues, m_d (UMLObjectPrivate*)
      │                 ObjectType enum: 28 values
      │                 28 isUML*() / asUML*() pairs
      │
      ├── UMLCanvasObject  (A)  — adds subordinate list m_List + association-end management
      │    │                      Fields: mutable UMLObjectList m_List
      │    │
      │    ├── UMLPackage  (A)  — adds standalone-object container m_objects
      │    │    │                 Fields: UMLObjectList m_objects
      │    │    │
      │    │    ├── UMLFolder  (C)  — adds diagram list
      │    │    │    │                Fields: m_localName, m_folderFile, UMLViewList m_diagrams
      │    │    │    │                Friend: UMLDoc
      │    │    │    │
      │    │    ├── UMLClassifier  (A)  — adds attributes, operations, templates
      │    │    │    │                     Fields: m_pClassAssoc
      │    │    │    │
      │    │    │    ├── UMLEnum  (C)  — adds enum literal management
      │    │    │    │
      │    │    │    ├── UMLDatatype  (C)  — adds m_isRef, m_isActive, originType
      │    │    │    │                       Fields: m_isRef, m_isActive
      │    │    │    │
      │    │    │    └── UMLEntity  (C)  — adds PrimaryKey + entity constraint management
      │    │    │                         Fields: m_PrimaryKey (UMLUniqueConstraint*)
      │    │    │
      │    │    ├── UMLComponent  (C)  — adds executable flag
      │    │    │                        Fields: m_executable
      │    │    │
      │    │    └── UMLArtifact  (C)  — adds draw-as-type (file/library/table)
      │    │                           Fields: m_drawAsType (Draw_Type)
      │    │
      │    ├── UMLActor  (C)  — leaf, no added fields
      │    ├── UMLUseCase  (C)  — leaf, no added fields
      │    ├── UMLNode  (C)  — leaf, no added fields
      │    ├── UMLPort  (C)  — leaf, no added fields
      │    ├── UMLCategory  (C)  — Disjoint/Overlapping/Union
      │    │                       Fields: m_CategoryType
      │    │
      │    └── UMLInstance  (C)  — references classifier via m_pSecondary
      │                           Auto-slaves instance attributes
      │
      ├── UMLClassifierListItem  (A)  — has setType/getType (uses m_pSecondary for type ref)
      │    │                            Pure virtual clone()
      │    │
      │    ├── UMLAttribute  (C)  — adds initialValue, ParmKind
      │    │    │                   Fields: m_InitialValue, m_ParmKind
      │    │    │
      │    │    └── UMLEntityAttribute  (C)  — adds indexType, autoIncrement, null
      │    │         │                         Fields: m_indexType, m_values,
      │    │         │                                 m_attributes, m_autoIncrement, m_null
      │    │
      │    ├── UMLOperation  (C)  — adds args list, const/override/final/virtual/inline
      │    │    │                   Fields: m_returnId, m_args (UMLAttributeList),
      │    │    │                           m_bConst, m_bOverride, m_bFinal,
      │    │    │                           m_bVirtual, m_bInline, m_Code
      │    │
      │    ├── UMLTemplate  (C)  — simple name-type pair
      │    │
      │    ├── UMLEnumLiteral  (C)  — adds constant value
      │    │                         Fields: m_Value
      │    │
      │    └── UMLEntityConstraint  (A)  — base for constraint types
      │         │                         Pure virtual clone()
      │         │
      │         ├── UMLUniqueConstraint  (C)  — list of entity attributes
      │         │                               Fields: m_EntityAttributeList
      │         │
      │         ├── UMLForeignKeyConstraint  (C)  — referenced entity + attribute mapping
      │         │    │                              Fields: m_ReferencedEntity,
      │         │    │                                      m_pReferencedEntityID,
      │         │    │                                      m_pEntityAttributeIDMap,
      │         │    │                                      m_AttributeMap,
      │         │    │                                      m_UpdateAction, m_DeleteAction
      │         │
      │         └── UMLCheckConstraint  (C)  — adds check condition string
      │                                       Fields: m_CheckCondition
      │
      ├── UMLAssociation  (C)  — owns 2 UMLRole objects, 25+ association types
      │    │                    Fields: m_pRole[2], m_AssocType, m_Name,
      │    │                            m_bOldLoadMode, nrof_parent_widgets
      │    │                    clone() returns nullptr — interface contract violation
      │    │
      │    └── UMLRole  (C)  — pointer to participant object, multiplicity, changeability
      │                        Fields: m_pAssoc, m_role, m_Multi, m_Changeability
      │                        clone() returns nullptr — interface contract violation
      │
      ├── UMLStereotype  (C)  — reference-counted, owned by UMLDoc::m_stereoList
      │    │                    Fields: m_refCount, m_attrDefs (AttributeDefs)
      │    │                    umlPackage() returns nullptr (special owner)
      │
      └── UMLInstanceAttribute  (C)  — value + pointer to owning UMLAttribute
                                       Fields: m_value
                                       Uses m_pSecondary for attribute pointer
```

### Inheritance Depth

| Depth | Classes |
|-------|---------|
| 1 (QObject → X) | UMLObject |
| 2 | UMLCanvasObject, UMLClassifierListItem, UMLAssociation, UMLStereotype, UMLInstanceAttribute |
| 3 | UMLPackage, UMLActor, UMLUseCase, UMLNode, UMLPort, UMLCategory, UMLInstance, UMLAttribute, UMLOperation, UMLTemplate, UMLEnumLiteral, UMLEntityConstraint, UMLRole |
| 4 | UMLFolder, UMLClassifier, UMLComponent, UMLArtifact, UMLEntityAttribute, UMLUniqueConstraint, UMLForeignKeyConstraint, UMLCheckConstraint |
| 5 | UMLEnum, UMLDatatype, UMLEntity |

Deepest chain: `QObject → UMLObject → UMLCanvasObject → UMLPackage → UMLClassifier → UMLEntity` (6 levels including QObject).

---

## 2. Class-by-Class Role Descriptions

### 2.1 UMLObject (Root)

**File:** `umlobject.h/cpp` (1476 lines — the largest model file)

**Role:** Abstract base for every model element. Provides:
- **Identity:** `m_nId` (Uml::ID::Type = `std::string`), managed by `UniqueID::gen()`
- **Naming:** `m_name`, `setName()`, `setNameCmd()` (undoable), `fullyQualifiedName()`
- **Type discrimination:** `m_BaseType` (ObjectType enum), 28 `isUML*()` predicates, 28 `asUML*()` cast wrappers
- **Visibility:** `m_visibility` (Public/Private/Protected/Implementation)
- **Stereotype:** `m_pStereotype` (QPointer<UMLStereotype>), plus `m_TaggedValues`
- **Documentation:** `m_Doc`
- **Abstract/Static flags:** `m_bAbstract`, `m_bStatic`
- **Secondary reference:** `m_pSecondary` — a generic QPointer<UMLObject> described as "only used by a few classes" but stored here because "of inheritance graph disjunctness". Used by:
  - `UMLClassifierListItem` subclasses → the type object
  - `UMLInstance` → the classifier
  - `UMLInstanceAttribute` → the UMLAttribute
  - `UMLDatatype` → originType
  - `UMLRole` → the participant object
  - `UMLOperation` → the return type
  - Also transiently during XMI loading for stereotype resolution
- **Secondary ID resolution:** `m_SecondaryId` + `m_SecondaryFallback` for deferred reference resolution during XMI load, with a fallback mechanism for Rose import.
- **Undo integration:** `setNameCmd()`, `setVisibilityCmd()`, `setStereotypeCmd()` push undo commands.
- **XMI:** `saveToXMI()` / `loadFromXMI()` with `save1()` / `load1()` template method pattern.
- **Qt:** Q_OBJECT with `modified()` signal; registered in `ObjectsModel` on construction.
- **D-pointer:** `UMLObjectPrivate* m_d` containing a single `bool isSaved`.

**Design smell:** The class is 1476 lines, has 28 type discriminators, carries `m_pSecondary` as a "universal slot", and depends on `UMLApp`, `UMLDoc`, `Object_Factory`, etc. — violating the single-responsibility principle.

### 2.2 UMLCanvasObject

**File:** `umlcanvasobject.h/cpp`

**Role:** Base for all diagram-displayed elements. Adds:
- **Subordinate list:** `mutable UMLObjectList m_List` — stores child objects (attributes, operations, templates, association ends, etc.)
- **Association end management:** `addAssociationEnd()`, `removeAssociationEnd()`, `getAssociations()`, `getSuperClasses()`, `getSubClasses()`, etc.

**Known issue (self-admitted in code):**
```
// @todo Only a pointer to the appropriate association end object
//       (UMLRole) should be saved here, not the entire UMLAssociation.
```

### 2.3 UMLPackage

**File:** `umlpackage.h/cpp`

**Role:** Namespace container. Adds:
- **Standalone object container:** `UMLObjectList m_objects` — stores classifiers, packages, etc. that have independent existence.
- **Query methods:** `findObject()`, `findObjectById()`, `appendPackages()`, `appendClassifiers()`, `appendClassesAndInterfaces()`, `appendEntities()`
- **Ownership:** The package owns objects in `m_objects` (distinct from subordinates in `m_List`).

**Design note:** The comment in the header says `m_objects` _could be merged into `m_List`_ — the distinction is semantic, not structural.

### 2.4 UMLFolder

**File:** `umlfolder.h/cpp`

**Role:** Top-level organizational unit. Adds:
- **Diagram storage:** `UMLViewList m_diagrams` — owns the views for this folder
- **Localization:** `m_localName` for i18n of predefined folders
- **Submodel file:** `m_folderFile` — if non-empty, this folder is saved as a separate XMI file
- **Options:** `setViewOptions()` propagates settings to child views

UMLDoc has 5 fixed instances for Logical/UseCase/Component/Deployment/EntityRelationship folders, plus user-created ones.

### 2.5 UMLClassifier

**File:** `umlclassifier.h/cpp`

**Role:** The central modeling class for classes and interfaces. Adds:
- **Attributes:** `addAttribute()`, `removeAttribute()`, `getAttributeList()`, `createAttribute()`
- **Operations:** `addOperation()`, `removeOperation()`, `findOperations()`, `checkOperationSignature()`, `createOperation()`
- **Templates:** `addTemplate()`, `removeTemplate()`, `findTemplate()`, `getTemplateList()`
- **ClassifierType enum:** `ALL`, `CLASS`, `INTERFACE`, `DATATYPE`
- **Inheritance queries:** `findSuperClassConcepts()`, `findSubClassConcepts()`, `hasAbstractOps()`, `hasAssociations()`, etc.
- **Overrides `setBaseType()`** — can be `ot_Class` or `ot_Interface`
- **`m_pClassAssoc`** — an internal association pointer

**Signals:** `operationAdded/Removed`, `templateAdded/Removed`, `attributeAdded/Removed`

### 2.6 UMLEnum

**File:** `umlenum.h/cpp`

**Role:** Extends classifier with enum literal management. Adds:
- `createEnumLiteral()`, `addEnumLiteral()`, `removeEnumLiteral()`, `enumLiterals()`
- **Signals:** `enumLiteralAdded/Removed`

### 2.7 UMLDatatype

**File:** `umldatatype.h/cpp`

**Role:** Primitive or reference datatype. Adds:
- `m_isRef` — is this a reference type?
- `m_isActive` — is this active?
- `originType()` / `setOriginType()` — uses `m_pSecondary` to point to the original classifier
- Used for programming-language primitive types (int, bool, string, etc.)

### 2.8 UMLEntity

**File:** `umlentity.h/cpp`

**Role:** Database table entity. Adds:
- **Primary key:** `m_PrimaryKey` — distinguished UMLUniqueConstraint
- **Entity attribute management:** `addEntityAttribute()`, `removeEntityAttribute()`, `getEntityAttributes()`
- **Constraint management:** `addConstraint()`, `removeConstraint()` for Unique/FK/Check
- **Factory methods:** `createUniqueConstraint()`, `createForeignKeyConstraint()`, `createCheckConstraint()`

### 2.9 UMLComponent / UMLArtifact

**Component:** Adds `m_executable` flag. Package-like (can nest).
**Artifact:** Adds `m_drawAsType` (defaultDraw, file, library, table). Package-like.

Both inherit from UMLPackage, not UMLCanvasObject directly, meaning they can contain nested elements.

### 2.10 UMLActor / UMLUseCase / UMLNode / UMLPort

All are thin subclasses of UMLCanvasObject with no added fields. They exist essentially as type tags to satisfy the ObjectType enum. Each overrides `clone()`, `saveToXMI()`, `load1()`, and optionally `init()`.

### 2.11 UMLCategory

**File:** `umlcategory.h/cpp`

**Role:** Represents a UML category (generalization set). Has:
- `m_CategoryType`: `ct_Disjoint_Specialisation`, `ct_Overlapping_Specialisation`, `ct_Union`

### 2.12 UMLInstance

**File:** `umlinstance.h/cpp`

**Role:** Represents an instance in an object diagram. Key behaviors:
- Uses `m_pSecondary` to reference the classifier
- Auto-creates/deletes `UMLInstanceAttribute` objects to mirror classifier attributes
- Connects to classifier's `attributeAdded/Removed` signals
- Notation: `instanceName : classifierName` (underlined)

### 2.13 UMLClassifierListItem (Abstract)

**File:** `umlclassifierlistitem.h/cpp`

**Role:** Base for items that live in classifier lists (attributes, operations, templates, enum literals, constraints).
- `setType(UMLObject*)` / `getType()` — uses `m_pSecondary` for the type reference
- `getTypeName()` — resolves the type's name
- Pure virtual `clone()` — though UMLEntityConstraint also declares it pure, and UMLAssociation/UMLRole break the contract by returning nullptr

### 2.14 UMLAttribute

**File:** `umlattribute.h/cpp`

**Role:** A programming-language attribute/field. Adds:
- `m_InitialValue` — default value string
- `m_ParmKind` — parameter direction (In/InOut/Out)
- `getTemplateParams()` — template parameter resolution

### 2.15 UMLEntityAttribute

**File:** `umlentityattribute.h/cpp`

**Role:** A database column. Extends UMLAttribute with:
- `m_indexType`: DBIndex_Type (None/Primary/Index/Unique)
- `m_values`, `m_attributes`: SQL column modifiers
- `m_autoIncrement`, `m_null`: boolean flags

### 2.16 UMLOperation

**File:** `umloperation.h/cpp`

**Role:** A method/operation. Adds:
- `m_args`: parameter list (UMLAttributeList)
- `m_returnId`: return type xmi.id
- 5 boolean flags: `m_bConst`, `m_bOverride`, `m_bFinal`, `m_bVirtual`, `m_bInline`
- `m_Code`: source code body
- `isConstructorOperation()` / `isDestructorOperation()` / `isLifeOperation()` — heuristic checks

### 2.17 UMLTemplate

**File:** `umltemplate.h/cpp`

**Role:** A template/generic parameter. Simple name-type pair where the type defaults to `"class"`. Uses `m_pSecondary` for the type.

### 2.18 UMLEnumLiteral

**File:** `umlenumliteral.h/cpp`

**Role:** A named value in an enum. Adds `m_Value` for explicit numeric/string assignment.

### 2.19 UMLEntityConstraint (Abstract)

**File:** `umlentityconstraint.h/cpp`

**Role:** Base for database constraints. Pure virtual `clone()`.

#### 2.19.1 UMLUniqueConstraint
- Holds `m_EntityAttributeList` — which columns form the unique constraint
- `hasEntityAttribute()`, `addEntityAttribute()`, `removeEntityAttribute()`

#### 2.19.2 UMLForeignKeyConstraint
- References `m_ReferencedEntity` (UMLEntity*)
- Maps local attributes to referenced attributes via `m_AttributeMap` (QMap)
- `m_UpdateAction`, `m_DeleteAction`: `UpdateDeleteAction` enum (NoAction/Restrict/Cascade/SetNull/SetDefault)
- Forward-reference resolution via `m_pReferencedEntityID` and `m_pEntityAttributeIDMap`

#### 2.19.3 UMLCheckConstraint
- Adds `m_CheckCondition` — a string representing the SQL check expression

### 2.20 UMLAssociation

**File:** `umlassociation.h/cpp`

**Role:** Represents any relationship between two model elements.
- Owns 2 `UMLRole` objects (`m_pRole[0]`, `m_pRole[1]`)
- `m_AssocType`: one of ~20 association types (see Section 4)
- `nrof_parent_widgets`: reference count of AssociationWidget instances
- `clone()` returns `nullptr` — "not implemented"
- XMI save/load is delegated to roles

### 2.21 UMLRole

**File:** `umlrole.h/cpp`

**Role:** An endpoint/role in an association.
- `m_pAssoc`: back-pointer to parent UMLAssociation
- `m_role`: A or B
- `m_Multi`: multiplicity string
- `m_Changeability`: Changeable/Frozen/AddOnly
- Participant object stored in `m_pSecondary` (inherited)
- `clone()` returns `nullptr`

### 2.22 UMLStereotype

**File:** `umlstereotype.h/cpp`

**Role:** Extension mechanism for UML. Special ownership model:
- Reference-counted (`m_refCount`), externally managed
- Owned by `UMLDoc::m_stereoList`, not by any UMLPackage
- `umlPackage()` returns `nullptr` (cannot use setUMLPackage)
- **AttributeDefs:** `QVector<AttributeDef>` — name/type/default triples
- `m_TaggedValues` in UMLObject stores concrete values corresponding to these defs

### 2.23 UMLInstanceAttribute

**File:** `umlinstanceattribute.h/cpp`

**Role:** Concrete value for an instance attribute. Uses:
- `m_pSecondary` → the `UMLAttribute` it corresponds to
- `m_value` → the concrete value string

### 2.24 List Types

There are **12 list type definitions** in separate files (see [Section 7.3](#73-12-separate-list-type-headers)):

| List Type | Element Type | Style |
|-----------|-------------|-------|
| `UMLObjectList` | `QPointer<UMLObject>` | Class (with clone/copyInto) |
| `UMLAssociationList` | `UMLAssociation*` | `typedef QList` |
| `UMLClassifierList` | `UMLClassifier*` | `typedef QList` |
| `UMLAttributeList` | `UMLAttribute*` | Class |
| `UMLOperationList` | `UMLOperation*` | `typedef QList` |
| `UMLTemplateList` | `UMLTemplate*` | `typedef QList` |
| `UMLEnumLiteralList` | `UMLEnumLiteral*` | `typedef QList` |
| `UMLEntityList` | `UMLEntity*` | `typedef QList` |
| `UMLEntityAttributeList` | `UMLEntityAttribute*` | Class |
| `UMLEntityConstraintList` | `UMLEntityConstraint*` | Class |
| `UMLClassifierListItemList` | `UMLClassifierListItem*` | Class |
| `UMLStereotypeList` | `UMLStereotype*` | `typedef QList` |
| `UMLPackageList` | `UMLPackage*` | `typedef QList` |

Inconsistency: Some are `typedef QList<T*>`, some are classes extending `QList<T*>` with clone/copyInto. `UMLObjectList` wraps `QPointer<UMLObject>` while most others use raw pointers.

---

## 3. UML Entity Taxonomy

### 3.1 Structural Classifiers (package-like, can own subordinates)

| Entity Type | ObjectType | C++ Class | Diagram | Adds |
|------------|------------|-----------|---------|------|
| Class | `ot_Class` | UMLClassifier | Class | attributes, operations, templates |
| Interface | `ot_Interface` | UMLClassifier | Class | same as Class (discriminated by baseType) |
| Enumeration | `ot_Enum` | UMLEnum | Class | enum literals |
| Datatype | `ot_Datatype` | UMLDatatype | Class | isRef, isActive, originType |
| Entity | `ot_Entity` | UMLEntity | Entity-relationship | PrimaryKey, constraints |

### 3.2 Package-like Entities (can nest)

| Entity Type | ObjectType | C++ Class | Diagram |
|------------|------------|-----------|---------|
| Package | `ot_Package` | UMLPackage | Class (as folder) |
| Folder | `ot_Folder` | UMLFolder | Tree View |
| Component | `ot_Component` | UMLComponent | Component |
| Artifact | `ot_Artifact` | UMLArtifact | Deployment |

### 3.3 Diagram Leaf Nodes (canvas objects, no nesting)

| Entity Type | ObjectType | C++ Class | Diagram |
|------------|------------|-----------|---------|
| Actor | `ot_Actor` | UMLActor | Use Case |
| Use Case | `ot_UseCase` | UMLUseCase | Use Case |
| Node | `ot_Node` | UMLNode | Deployment |
| Port | `ot_Port` | UMLPort | Composite Structure |
| Category | `ot_Category` | UMLCategory | Class (generalization set) |
| Instance | `ot_Instance` | UMLInstance | Object |

### 3.4 Classifier Children (owned by classifiers)

| Entity Type | ObjectType | C++ Class | Owner |
|------------|------------|-----------|-------|
| Attribute | `ot_Attribute` | UMLAttribute | UMLClassifier |
| Operation | `ot_Operation` | UMLOperation | UMLClassifier |
| Template | `ot_Template` | UMLTemplate | UMLClassifier |
| Enum Literal | `ot_EnumLiteral` | UMLEnumLiteral | UMLEnum |
| Entity Attribute | `ot_EntityAttribute` | UMLEntityAttribute | UMLEntity |
| Unique Constraint | `ot_UniqueConstraint` | UMLUniqueConstraint | UMLEntity |
| Foreign Key Constraint | `ot_ForeignKeyConstraint` | UMLForeignKeyConstraint | UMLEntity |
| Check Constraint | `ot_CheckConstraint` | UMLCheckConstraint | UMLEntity |

### 3.5 Relationship Objects

| Entity Type | ObjectType | C++ Class | Notes |
|------------|------------|-----------|-------|
| Association | `ot_Association` | UMLAssociation | 25+ subtypes |
| Role | `ot_Role` | UMLRole | Endpoint of association |

### 3.6 Model Infrastructure

| Entity Type | ObjectType | C++ Class | Notes |
|------------|------------|-----------|-------|
| Stereotype | `ot_Stereotype` | UMLStereotype | Reference-counted, special ownership |
| Instance Attribute | `ot_InstanceAttribute` | UMLInstanceAttribute | Value holder, auto-created |

### 3.7 Standalone vs. Subordinate Distinction

The codebase distinguishes two categories of objects:

1. **Standalone objects** — have independent existence, stored in `UMLPackage::m_objects`:
   - All classifiers, packages, folders, components, artifacts, actors, use cases, nodes, ports, categories, instances
   - These are visible in the tree view and can be created/deleted by the user

2. **Subordinate objects** — cannot exist independently, stored in `UMLCanvasObject::m_List`:
   - Attributes, operations, templates, enum literals, entity attributes, constraints
   - These are slaved to their parent classifier

3. **Special objects** — neither fully standalone nor subordinate in the usual sense:
   - Associations: exist independently but live through AssociationWidget references
   - Stereotypes: owned globally by UMLDoc
   - Roles: owned by UMLAssociation
   - Instance attributes: auto-created mirrors of classifier attributes

---

## 4. Relationship Types and Modelling

### 4.1 AssociationType Enum (25+ Values)

From `basictypes.h`, `Uml::AssociationType::Enum`:

```cpp
enum Enum {
    Generalization       = 500,  // UML: generalization (inheritance)
    Aggregation,                 // UML: shared aggregation
    Dependency,                  // UML: dependency
    Association,                 // UML: plain association
    Association_Self,            // UML: reflexive association
    Coll_Mesg_Async,             // Sequence: asynchronous message
    Seq_Message,                 // Sequence: sequential message
    Coll_Mesg_Self,              // Sequence: self-async message
    Seq_Message_Self,            // Sequence: self-seq message
    Containment,                 // UML: containment (deprecated in UML2)
    Composition,                 // UML: composite aggregation
    Realization,                 // UML: realization (interface impl)
    UniAssociation,              // UML: directed association
    Anchor,                      // Diagram: note anchor
    State,                       // State: state transition
    Activity,                    // Activity: flow
    Exception,                   // UML: exception (deprecated)
    Category2Parent,             // Category: to parent
    Child2Category,              // Category: from child
    Relationship,                // ER: entity relationship
    Coll_Mesg_Sync,              // Sequence: synchronous (collaboration)
    Reserved,                    // Sentinel
    Unknown = -1
};
```

**Critical observation:** This enum **mixes UML structural relationships** (Generalization, Aggregation, Association) with **sequence/collaboration diagram message types** (Seq_Message, Coll_Mesg_Async, etc.) and **diagram notation elements** (Anchor, State, Activity). This conflation propagates through the entire codebase.

### 4.2 How Relationships Are Modeled

```
UMLAssociation
 ├── UMLRole[0] ──→ UMLObject (participant A)
 │      └── multiplicity, changeability, role name, role doc
 ├── UMLRole[1] ──→ UMLObject (participant B)
 │      └── multiplicity, changeability, role name, role doc
 └── AssociationType
```

| Relationship | AssociationType | AssociationWidget Type |
|-------------|----------------|----------------------|
| Inheritance | Generalization | GeneralizationWidget |
| Interface implementation | Realization | RealizationWidget |
| Plain association | Association | AssociationWidget |
| Directed association | UniAssociation | AssociationWidget (directed) |
| Shared aggregation | Aggregation | AssociationWidget (aggregation) |
| Composition | Composition | AssociationWidget (composition) |
| Dependency | Dependency | AssociationWidget (dependency) |
| Note link | Anchor | NoteWidget |

**Key observation:** The AssociationWidget collaborator (not analyzed in detail here) mirrors UMLAssociation and handles the graphical representation. The TODO in UMLAssociation comments notes: _the UMLAssociation should continue to exist when no AssociationWidget exists. We do not yet have the means to delete the UMLAssociation._

### 4.3 Association-End Registration

UMLCanvasObject maintains `m_List` of association ends. Each `UMLAssociation` is registered with both participant objects via `addAssociationEnd()`. The association object itself is stored in the list. The TODO notes this should store `UMLRole` pointers instead.

### 4.4 Generalization Hierarchy Queries

`UMLCanvasObject::getSuperClasses()` traverses association ends looking for Generalization/Realization associations where `this` is Role B (the subclass/implementor). `getSubClasses()` does the inverse. `UMLClassifier::findSuperClassConcepts()` / `findSubClassConcepts()` provide filtered access.

### 4.5 Container-Contained vs. Association

The model uses two distinct mechanisms for "has-a" relationships:
1. **Container/contained** via UMLPackage::m_objects / UMLCanvasObject::m_List — strong ownership, XMI nesting
2. **Associations** via UMLAssociation + UMLRole — loose coupling, cross-references by ID

---

## 5. Ownership and Containment Model

### 5.1 Qt Object Tree

Every `UMLObject` is a `QObject` and participates in Qt's parent-child tree:
- `UMLDoc` is the root object (indirectly)
- Parent is set via `QObject` constructor argument (`umlParent()` returns the parent)
- Qt manages deletion: when a parent is destroyed, children are destroyed

### 5.2 Semantic Ownership

Parallel to the Qt tree, there is semantic containment:

```
UMLDoc
 └── UMLFolder (predefined: Logical, UseCase, Component, Deployment, ER)
      └── UMLPackage (nested packages, classifiers, etc.)
           ├── m_objects → standalone objects
           │    ├── UMLClassifier
           │    │    └── m_List → attributes, operations, templates
           │    ├── UMLActor
           │    ├── UMLUseCase
           │    └── ...
           ├── m_List → association ends
           └── UMLFolder (user-created)
                └── UMLView (diagrams) — via m_diagrams

UMLDoc (special)
 └── m_stereoList → UMLStereotype objects (not in any package)
```

### 5.3 Ownership Rules

| Owner | Owned Objects | Storage |
|-------|--------------|---------|
| UMLDoc | UMLFolder (top-level), UMLStereotype, UMLView | m_folders, m_stereoList |
| UMLFolder | UMLView (diagrams) | m_diagrams |
| UMLPackage | Standalone objects (classifiers, packages, actors, etc.) | m_objects |
| UMLCanvasObject | Subordinate objects + association ends | m_List |
| UMLClassifier | Attributes, Operations, Templates | m_List (inherited) |
| UMLEnum | Enum Literals | m_List (inherited) |
| UMLEntity | Entity Attributes, Constraints | m_List (inherited) |
| UMLAssociation | UMLRole (2) | m_pRole[0/1] |

### 5.4 Registration in ObjectsModel

Every UMLObject, upon construction, registers itself in `UMLDoc::objectsModel()` — a Qt `QAbstractItemModel` that provides a flattened, uniform view of all model objects. This is a **friend of UMLObject** (accesses private members) and is used by the tree view.

### 5.5 Dual-Ownership Problem

Objects are simultaneously owned by:
1. **Qt parent chain** (determines memory lifetime)
2. **UMLPackage::m_objects** (determines semantic namespace)
3. **ObjectsModel** (determines tree-view presence)

These can become inconsistent. In particular, `QObject` parenting doesn't always match semantic containment: a stereotype is parented to its using object but owned by UMLDoc's stereotype list.

### 5.6 Association Lifecycle

Associations have a peculiar lifecycle:
- Created independently, not as children of any package
- Registered with both participants via `addAssociationEnd()`
- Destroyed when the last `AssociationWidget` referencing them is destroyed (tracked via `nrof_parent_widgets`)
- This is described as a TODO: "We do not yet have the means to delete the UMLAssociation"

---

## 6. XMI Serialization Patterns

### 6.1 Template Method

```
UMLObject::saveToXMI()       [virtual, default writes common attrs]
   └── save1()               [writes start element + common attrs]
   └── save1end()            [writes end element]
   ↓  (overridden by)
UMLClassifier::saveToXMI()   [writes classifier-specific content]
UMLOperation::saveToXMI()    [writes operation-specific content]
   ...

UMLObject::loadFromXMI()     [virtual, dispatches to load1()]
   ↓  (overridden by)
   load1()                   [called by UMLObject::loadFromXMI after basic setup]
```

### 6.2 ID-Based Reference Resolution

The XMI loading process uses two-phase construction:
1. **Phase 1 — loadFromXMI/load1:** Create objects, store ID references as strings (`m_SecondaryId`, `m_SecondaryFallback`)
2. **Phase 2 — resolveRef():** Walk all objects and resolve string IDs to pointers via `UMLDoc::findObjectById()`

This is a two-pass approach necessary because objects may be loaded in any order and can reference not-yet-loaded objects.

### 6.3 Stereotype Resolution

During `resolveRef()`, if `m_SecondaryId` resolves to a `UMLStereotype`, it is moved from `m_pSecondary` to `m_pStereotype` and `m_pSecondary` is set to nullptr. This is an ad-hoc type-dependent behavior baked into the root class.

---

## 7. Technical Debt Inventory

### 7.1 Manual RTTI (28 isUML*/asUML* Pairs) — SEVERITY: HIGH

`UMLObject` defines 28 `isUML*()` predicates and 56 const/non-const `asUML*()` casters (84 methods total). These are essentially manual implementations of `dynamic_cast` using an enum discriminator. This is:
- Not extensible without modifying UMLObject
- Error-prone (casts can return nullptr if type is wrong — but callers assume they don't)
- Pollutes the root class interface
- 1544+ lines of trivial implementations

### 7.2 m_pSecondary — Generic Back-Pointer — SEVERITY: HIGH

A single `QPointer<UMLObject>` used for at least 5 semantically distinct purposes:
- `UMLClassifierListItem::getType()` — the type of an attribute/operation/template
- `UMLInstance::classifier()` — the classifier being instantiated
- `UMLInstanceAttribute::getAttribute()` — the owning attribute
- `UMLDatatype::originType()` — the original classifier
- `UMLRole::object()` — the participant object
- Plus transient use during XMI stereotype resolution

The comment in the header says: _"Only a few of the classes inheriting from UMLObject use this. However, it needs to be here because of inheritance graph disjunctness."_ This is a clear sign that the inheritance hierarchy doesn't capture the actual type relationships.

### 7.3 12 Separate List Type Headers — SEVERITY: MEDIUM

12 separate files for type aliases and thin wrappers around `QList`. Some are typedefs, some are classes with copyInto/clone. The inconsistency between `QPointer<UMLObject>` (UMLObjectList) and raw pointers (all others) is a latent memory-safety issue.

### 7.4 Clone Contract Violation — SEVERITY: HIGH

The `clone()` method is declared virtual in UMLObject (returns `new` object) but:
- `UMLAssociation::clone()` returns `nullptr`
- `UMLRole::clone()` returns `nullptr`
- `UMLObject::clone()` returns `nullptr` (base)
- `UMLEntityConstraint::clone()` is pure virtual (correct, but subclasses implement it)

This breaks the Liskov substitution principle — callers cannot safely call `clone()` on arbitrary objects.

### 7.5 CanvasObject Stores Full Association Instead of Role — SEVERITY: MEDIUM

`UMLCanvasObject::m_List` stores `UMLAssociation*` directly, but logically only the `UMLRole` (endpoint) belongs to the participant. The code even admits this as a TODO.

### 7.6 AssociationType Mixes Concerns — SEVERITY: HIGH

The `Uml::AssociationType::Enum` mixes structural UML relationships, diagram messages, and notation elements. This enum is used in at least 50+ locations across the codebase (including AssociationWidget, various importers/exporters). Adding a new UML relationship type requires potentially updating all of them.

### 7.7 Boolean Flag Proliferation — SEVERITY: MEDIUM

`UMLOperation` has 5 separate `bool` fields (`m_bConst`, `m_bOverride`, `m_bFinal`, `m_bVirtual`, `m_bInline`). `UMLEntityAttribute` duplicates boolean pattern. These should be bitflags.

### 7.8 ObjectsModel is Friend of UMLObject — SEVERITY: MEDIUM

`ObjectsModel` accesses `UMLObject` private members directly (friend declaration). This bypasses the public API and couples the model layer to the Qt model/view framework.

### 7.9 Qt Dependency in Model Layer — SEVERITY: HIGH

The entire model layer inherits from `QObject`, uses `QList`, `QString`, `QPointer`, `QDomElement`, `QXmlStreamWriter`, `Q_OBJECT`, `Q_SIGNALS`, `Q_SLOTS`, and `Q_ENUMS`. This makes the model:
- Untestable without Qt
- Unusable outside a Qt event loop
- Tightly coupled to Qt's memory management (QObject parent tree)
- Non-reusable in non-GUI contexts (e.g., command-line tools)

### 7.10 UMLObject::init() Called from All Constructors — SEVERITY: LOW

The `init()` method, called from all constructors, does global lookups (`UMLApp::app()->document()->...`). This creates hidden dependencies on runtime state during construction.

### 7.11 UMLObject.cpp is 1476 Lines — SEVERITY: MEDIUM

The root class implementation file is excessively large. Much of the code is boilerplate getter/setter implementations for the 28 type check methods plus the resolveRef() logic.

### 7.12 Hidden Global State Dependencies — SEVERITY: HIGH

Constructors call `UMLApp::app()->document()->objectsModel()->add(this)`. Destructors call `UMLApp::app()->document()->objectsModel()->remove(this)`. This means:
- `UMLObject` cannot be instantiated without a fully initialized application
- Unit testing model classes requires the full application stack
- The model is not testable in isolation

---

## 8. Rust Recommendations

### 8.1 Replace Inheritance Hierarchy with Trait-Based Composition

**Problem:** The deep C++ inheritance tree creates coupling, prevents mixins, and forces all behavior through the root class.

**Recommendation:** Use trait-based composition:

```rust
/// Core identity trait (analogous to UMLObject)
pub trait UmlObject: Identifiable + Named {
    fn id(&self) -> UmlId;
    fn name(&self) -> &str;
    fn visibility(&self) -> Visibility;
    fn stereotype(&self) -> Option<&Stereotype>;
    fn documentation(&self) -> &str;
    fn tagged_values(&self) -> &[TaggedValue];
}

/// Can contain subordinate items (was UMLCanvasObject)
pub trait HasSubordinates {
    fn subordinates(&self) -> &[SubordinateRef];
    fn add_subordinate(&mut self, child: SubordinateRef);
}

/// Can contain standalone items (was UMLPackage)
pub trait HasOwnedObjects {
    fn owned_objects(&self) -> &[ObjectRef];
    fn add_object(&mut self, obj: ObjectRef);
}

/// Has attributes, operations, templates (was UMLClassifier)
pub trait Classifier: UmlObject + HasOwnedObjects {
    fn attributes(&self) -> &[Attribute];
    fn operations(&self) -> &[Operation];
    fn templates(&self) -> &[TemplateParameter];
}
```

### 8.2 Enum-Based Dispatch Instead of isUML*/asUML*

**Problem:** 28 `isUML*()` / `asUML*()` pairs are non-extensible, verbose, and pollute the root class.

**Recommendation:** Use a Rust enum for type-safe dispatch:

```rust
#[derive(Clone, Debug)]
pub enum UmlModelElement {
    // Structural classifiers
    Class(Box<UmlClass>),
    Interface(Box<UmlInterface>),
    Enumeration(Box<UmlEnumeration>),
    Datatype(Box<UmlDatatype>),
    Entity(Box<UmlEntity>),
    
    // Package-like
    Package(Box<UmlPackage>),
    Folder(Box<UmlFolder>),
    Component(Box<UmlComponent>),
    Artifact(Box<UmlArtifact>),
    
    // Leaf nodes
    Actor(Box<UmlActor>),
    UseCase(Box<UmlUseCase>),
    Node(Box<UmlNode>),
    Port(Box<UmlPort>),
    Category(Box<UmlCategory>),
    Instance(Box<UmlInstance>),
    
    // Relationships
    Association(Box<UmlAssociation>),
    Role(Box<UmlRole>),
    
    // Subordinate items
    Attribute(Box<UmlAttribute>),
    Operation(Box<UmlOperation>),
    Template(Box<UmlTemplate>),
    EnumLiteral(Box<UmlEnumLiteral>),
    EntityAttribute(Box<UmlEntityAttribute>),
    
    // Constraints
    UniqueConstraint(Box<UmlUniqueConstraint>),
    ForeignKeyConstraint(Box<UmlForeignKeyConstraint>),
    CheckConstraint(Box<UmlCheckConstraint>),
    
    // Infrastructure
    Stereotype(Box<Stereotype>),
    InstanceAttribute(Box<UmlInstanceAttribute>),
}

impl UmlModelElement {
    // No need for isUML*() — match on the enum
    pub fn name(&self) -> &str { /* dispatch */ }
    pub fn id(&self) -> UmlId { /* dispatch */ }
    // Visitors for type-specific operations
    pub fn accept(&self, visitor: &dyn UmlVisitor) { /* double dispatch */ }
}
```

### 8.3 Arena/Generational Index Allocation

**Problem:** QObject parent-chain memory management, global-state registration in constructors.

**Recommendation:** Use arena-based allocation with generational indices (e.g., `slotmap` or `generational-arena`):

```rust
use slotmap::SlotMap;

// Instead of raw pointers
pub type ObjectKey = slotmap::DefaultKey;

pub struct ModelArena {
    objects: SlotMap<ObjectKey, UmlModelElement>,
    // ... specialized slot maps for hot types
    classifiers: SecondaryMap<ObjectKey, ClassifierData>,
}

impl ModelArena {
    pub fn insert(&mut self, element: UmlModelElement) -> ObjectKey {
        let key = self.objects.insert(element);
        // Auto-register in type-specific maps
        key
    }
    pub fn get(&self, key: ObjectKey) -> Option<&UmlModelElement> {
        self.objects.get(key)
    }
    pub fn remove(&mut self, key: ObjectKey) {
        self.objects.remove(key);
        // Clean up references to this key
    }
}
```

Benefits:
- No dangling pointers (SlotMap detects stale keys at runtime)
- No global state in constructors
- Arenas can be tested independently
- Fast iteration (cache-friendly contiguous storage)
- Cheap IDs (single word) for XMI serialization

### 8.4 Standard Collections Instead of QList

**Problem:** 12+ custom list types, inconsistent patterns, QList overhead.

**Recommendation:** Use `Vec<T>` or typed wrappers:

```rust
// Single generic collection instead of 12 special-purpose lists
pub type ObjectList = Vec<ObjectKey>;
pub type AssociationList = Vec<AssociationKey>;
pub type AttributeList = Vec<AttributeKey>;
// or simply use Vec<Key> everywhere with type aliases

// For sorted/unique requirements, use BTreeSet/HashSet
```

### 8.5 Visitor Pattern for XMI Serialization

**Problem:** Virtual saveToXMI/loadFromXMI couples serialization to the model classes.

**Recommendation:** Implement XMI serialization via a visitor trait:

```rust
pub trait UmlVisitor {
    fn visit_class(&mut self, class: &UmlClass) -> Result<(), XmiError>;
    fn visit_interface(&mut self, iface: &UmlInterface) -> Result<(), XmiError>;
    fn visit_association(&mut self, assoc: &UmlAssociation) -> Result<(), XmiError>;
    // ... one per variant
    fn visit_element(&mut self, element: &UmlModelElement) -> Result<(), XmiError> {
        element.accept(self)
    }
}

// Concrete visitors:
pub struct XmiWriter<'a, W: Write> {
    writer: &'a mut xml::Writer<W>,
    arena: &'a ModelArena,
}

pub struct XmiReader<'a> {
    arena: &'a mut ModelArena,
    unresolved: Vec<UnresolvedRef>,
}
```

### 8.6 Replace m_pSecondary with Proper Sum Types

**Problem:** A single field serving 6+ semantically different purposes.

**Recommendation:** Use Enum + `Option<T>` where each variant holds exactly what it needs:

```rust
/// For UMLClassifierListItem (attribute, operation, template, etc.)
pub struct TypedItem {
    pub type_ref: Option<ObjectKey>,  // formerly m_pSecondary
    // ...
}

/// For UMLInstance
pub struct Instance {
    pub classifier: Option<ObjectKey>,  // formerly m_pSecondary
    pub attribute_values: Vec<InstanceAttributeValue>,
    // ...
}

/// For UMLDatatype
pub struct Datatype {
    pub origin_type: Option<ObjectKey>,  // formerly m_pSecondary
    pub is_reference: bool,
    pub is_active: bool,
    // ...
}
```

### 8.7 Bitflags for Boolean Flags

**Problem:** Multiple bool fields are scattered.

**Recommendation:** Use the `bitflags` crate:

```rust
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct OperationFlags: u8 {
        const CONST    = 0b0001;
        const OVERRIDE = 0b0010;
        const FINAL    = 0b0100;
        const VIRTUAL  = 0b1000;
    }
}

pub struct Operation {
    flags: OperationFlags,
    // vs. 5 separate bool fields
}
```

### 8.8 Replace ObjectType Enum with Rust Enum

**Problem:** C++ `ObjectType` uses magic numbers starting at 100, with `-1` for `Unknown`.

**Recommendation:** Rust enum (already aligned with recommendation 8.2):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    Actor,
    UseCase,
    Package,
    Interface,
    Datatype,
    Enumeration,
    Class,
    Instance,
    Association,
    Attribute,
    Operation,
    EnumLiteral,
    Template,
    Component,
    Artifact,
    Node,
    Stereotype,
    Role,
    Entity,
    EntityAttribute,
    Folder,
    EntityConstraint,
    UniqueConstraint,
    ForeignKeyConstraint,
    CheckConstraint,
    Category,
    Port,
    InstanceAttribute,
}

impl ObjectType {
    pub fn from_xmi(s: &str) -> Result<Self, ParseError> { /* ... */ }
    pub fn to_xmi(self) -> &'static str { /* ... */ }
}
```

### 8.9 Event System via Channels

**Problem:** Qt signals/slots and global `UMLApp::app()` coupling for model-change notification.

**Recommendation:** Message passing via channels/callbacks:

```rust
pub enum ModelEvent {
    ObjectCreated(ObjectKey),
    ObjectRemoved(ObjectKey),
    AttributeAdded(ObjectKey, ObjectKey),  // (classifier, attribute)
    AttributeRemoved(ObjectKey, ObjectKey),
    NameChanged(ObjectKey, String),
    // ...
}

pub struct ModelEventBus {
    sender: broadcast::Sender<ModelEvent>,
}

impl ModelEventBus {
    pub fn subscribe(&self) -> broadcast::Receiver<ModelEvent> {
        self.sender.subscribe()
    }
    pub fn emit(&self, event: ModelEvent) {
        let _ = self.sender.send(event);
    }
}
```

### 8.10 Separate Model from Presentation

**Problem:** Model classes reference `UMLApp`, `UMLDoc`, `ObjectsModel`, widgets, KDE i18n, and dialog classes.

**Recommendation:** The Rust model crate should be:
- **No dependencies on GUI frameworks** (no Qt, no GTK, no web framework)
- **Pure data + validation** — focusing on UML semantics
- **Serialization decoupled** via visitor pattern
- **Events via channels** (not callbacks into GUI)
- **Testable without any runtime setup**

---

## 9. Proposed Rust Type Hierarchy

### 9.1 Core Types

```rust
// === Identity ===
pub type UmlId = String;  // Or a newtype wrapper (could be uuid)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectKey(slotmap::DefaultKey);

// === Arenas ===
pub struct ModelArena {
    objects: SlotMap<ObjectKey, UmlModelElement>,
    stereotypes: SlotMap<ObjectKey, StereotypeData>,
}

// === Core Traits ===
pub trait Identifiable {
    fn key(&self) -> ObjectKey;
}
pub trait Named {
    fn name(&self) -> &str;
}

// === Visibility and Scope ===
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility { Public, Private, Protected, Implementation }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope { Instance, Classifier }

// === Stereotype ===
#[derive(Debug, Clone)]
pub struct StereotypeData {
    pub name: String,
    pub ref_count: usize,
    pub attribute_defs: Vec<StereotypeAttrDef>,
}

// === Main Model Enum ===
#[derive(Debug, Clone)]
pub enum UmlModelElement {
    // == Structural ==
    Class(UmlClass),
    Interface(UmlInterface),
    Enumeration(UmlEnumeration),
    Datatype(UmlDatatype),
    Entity(UmlEntity),

    // == Containers ==
    Package(UmlPackage),
    Folder(UmlFolder),

    // == Component/Deployment ==
    Component(UmlComponent),
    Artifact(UmlArtifact),

    // == Use Case ==
    Actor(UmlActor),
    UseCase(UmlUseCase),

    // == Deployment ==
    Node(UmlNode),
    Port(UmlPort),

    // == Generalization Sets ==
    Category(UmlCategory),

    // == Object Diagrams ==
    Instance(UmlInstance),

    // == Relationships ==
    Association(UmlAssociation),
    Role(UmlRole),

    // == Classifier Children ==
    Attribute(UmlAttribute),
    Operation(UmlOperation),
    TemplateParameter(UmlTemplateParameter),
    EnumLiteral(UmlEnumLiteral),
    EntityAttribute(UmlEntityAttribute),

    // == Constraints ==
    UniqueConstraint(UmlUniqueConstraint),
    ForeignKeyConstraint(UmlForeignKeyConstraint),
    CheckConstraint(UmlCheckConstraint),

    // == Infrastructure ==
    Stereotype(UmlStereotype),
    InstanceAttribute(UmlInstanceAttribute),
}
```

### 9.2 Structural Types

```rust
#[derive(Debug, Clone)]
pub struct UmlClass {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub is_abstract: bool,
    pub stereotype: Option<ObjectKey>,
    pub documentation: String,
    pub tagged_values: Vec<TaggedValue>,
    pub owned_objects: Vec<ObjectKey>,   // nested classifiers/packages
    pub subordinates: Vec<ObjectKey>,     // attributes, operations, templates
    pub associations: Vec<AssociationEnd>,
}

#[derive(Debug, Clone)]
pub struct UmlInterface {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype: Option<ObjectKey>,
    pub documentation: String,
    pub owned_objects: Vec<ObjectKey>,
    pub subordinates: Vec<ObjectKey>,     // operations, templates only (no attributes)
    pub associations: Vec<AssociationEnd>,
}

#[derive(Debug, Clone)]
pub struct UmlEnumeration {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype: Option<ObjectKey>,
    pub literals: Vec<ObjectKey>,          // enum literals
    pub owned_objects: Vec<ObjectKey>,
    pub associations: Vec<AssociationEnd>,
}

#[derive(Debug, Clone)]
pub struct UmlDatatype {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub is_reference: bool,
    pub is_active: bool,
    pub origin_type: Option<ObjectKey>,
    pub stereotype: Option<ObjectKey>,
}

#[derive(Debug, Clone)]
pub struct UmlEntity {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype: Option<ObjectKey>,
    pub entity_attributes: Vec<ObjectKey>,
    pub constraints: Vec<ObjectKey>,
    pub primary_key: Option<ObjectKey>,   // points to a UniqueConstraint
    pub owned_objects: Vec<ObjectKey>,
}
```

### 9.3 Container Types

```rust
#[derive(Debug, Clone)]
pub struct UmlPackage {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype: Option<ObjectKey>,
    pub owned_objects: Vec<ObjectKey>,
    pub associations: Vec<AssociationEnd>,
}

#[derive(Debug, Clone)]
pub struct UmlFolder {
    pub key: ObjectKey,
    pub name: String,
    pub local_name: Option<String>,       // i18n name for predefined folders
    pub folder_file: Option<String>,      // separate file for submodel
    pub owned_objects: Vec<ObjectKey>,
    pub diagrams: Vec<DiagramKey>,        // references to UMLView equivalents
}
```

### 9.4 Relationship Types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationType {
    // UML structural relationships
    Generalization,
    Aggregation,
    Composition,
    Association,
    DirectedAssociation,
    Dependency,
    Realization,
    Containment,
    Exception,
    
    // Generalization sets
    CategoryToParent,
    ChildToCategory,
    
    // Entity-Relationship
    Relationship,
    
    // Diagram notation
    Anchor,
    
    // (Separate from sequence/collaboration message types)
}

#[derive(Debug, Clone)]
pub struct UmlAssociation {
    pub key: ObjectKey,
    pub assoc_type: AssociationType,
    pub role_a: ObjectKey,      // points to UMLRole
    pub role_b: ObjectKey,      // points to UMLRole
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct UmlRole {
    pub key: ObjectKey,
    pub participant: ObjectKey,   // the UMLObject at this end
    pub multiplicity: String,
    pub role_name: String,
    pub role_doc: String,
    pub visibility: Visibility,
    pub changeability: Changeability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changeability { Changeable, Frozen, AddOnly }
```

### 9.5 Classifier Child Types

```rust
#[derive(Debug, Clone)]
pub struct UmlAttribute {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub type_ref: ObjectKey,             // the type (classifier/datatype)
    pub initial_value: Option<String>,
    pub is_static: bool,
    pub stereotype: Option<ObjectKey>,
}

#[derive(Debug, Clone)]
pub struct UmlOperation {
    pub key: ObjectKey,
    pub name: String,
    pub visibility: Visibility,
    pub return_type: Option<ObjectKey>,
    pub parameters: Vec<UmlParameter>,
    pub flags: OperationFlags,            // bitflags for const/override/final/virtual/inline
    pub source_code: Option<String>,
    pub is_abstract: bool,
    pub is_static: bool,
    pub stereotype: Option<ObjectKey>,
}

#[derive(Debug, Clone)]
pub struct UmlParameter {
    pub name: String,
    pub type_ref: ObjectKey,
    pub direction: ParameterDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterDirection { In, InOut, Out }

#[derive(Debug, Clone)]
pub struct UmlTemplateParameter {
    pub key: ObjectKey,
    pub name: String,
    pub type_ref: Option<ObjectKey>,
}

#[derive(Debug, Clone)]
pub struct UmlEnumLiteral {
    pub key: ObjectKey,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UmlEntityAttribute {
    pub key: ObjectKey,
    pub name: String,
    pub type_ref: ObjectKey,
    pub visibility: Visibility,
    pub initial_value: Option<String>,
    pub index_type: IndexType,
    pub auto_increment: bool,
    pub nullable: bool,
    pub attributes: String,   // SQL column modifiers
    pub values: String,       // allowed values
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType { None, Primary, Index, Unique }
```

### 9.6 Constraint Types

```rust
#[derive(Debug, Clone)]
pub struct UmlUniqueConstraint {
    pub key: ObjectKey,
    pub name: String,
    pub entity_attributes: Vec<ObjectKey>,   // columns in this constraint
}

#[derive(Debug, Clone)]
pub struct UmlForeignKeyConstraint {
    pub key: ObjectKey,
    pub name: String,
    pub referenced_entity: ObjectKey,
    pub column_mappings: Vec<(ObjectKey, ObjectKey)>,  // (local_attr, referenced_attr)
    pub update_action: ReferentialAction,
    pub delete_action: ReferentialAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferentialAction { NoAction, Restrict, Cascade, SetNull, SetDefault }

#[derive(Debug, Clone)]
pub struct UmlCheckConstraint {
    pub key: ObjectKey,
    pub name: String,
    pub condition: String,
}
```

### 9.7 Remaining Types

```rust
#[derive(Debug, Clone)]
pub struct UmlActor {
    pub key: ObjectKey,
    pub name: String,
    pub stereotype: Option<ObjectKey>,
    pub associations: Vec<AssociationEnd>,
}

#[derive(Debug, Clone)]
pub struct UmlUseCase {
    pub key: ObjectKey,
    pub name: String,
    pub stereotype: Option<ObjectKey>,
    pub associations: Vec<AssociationEnd>,
}

// ... similarly for UmlNode, UmlPort, UmlCategory, UmlComponent, UmlArtifact,
//     UmlInstance, UmlStereotype, UmlInstanceAttribute
```

---

## 10. Migration Considerations

### 10.1 Strategy: Bottom-Up Rewrite

Phase the migration in four stages:

**Phase 1 — Arena Model (Rust):**
- Implement `ModelArena` with slotmap
- Implement all model types as Rust structs (no traits yet)
- Implement XMI reader/writer via visitor
- Validate against existing XMI test files
- All pure data, no business logic, no GUI

**Phase 2 — Business Logic:**
- Add mutation methods (add/remove attributes, operations, etc.)
- Implement `resolve_refs()` for XMI loading
- Add constraint validation
- Port validation rules

**Phase 3 — Event System:**
- Add `ModelEventBus` using broadcast channels
- Port `ObjectsModel` equivalent as subscriber
- Ensure thread-safety (Rust ownership model prevents data races)

**Phase 4 — FFI Bridge:**
- Create a C FFI layer for gradual adoption
- Allow C++ code to use the Rust model via pointers + callback event handlers
- Or: rewrite the Qt GUI directly in Rust using a Rust GUI framework

### 10.2 Testing Strategy

| Layer | Test Approach | Scope |
|-------|--------------|-------|
| Model types | Unit tests with `assert_eq!` | 100% of struct methods |
| XMI roundtrip | Read XMI → validate structs → write → diff | All 28 entity types |
| Business logic | Mutation + verification | Attribute/operation/constraint CRUD |
| Edge cases | Null refs, cycles, unresolvable xmi.id | resolve_refs, delete propagation |
| Performance | Criterion benchmarks | Arena insertion, XMI parse, traversal |

### 10.3 Key Design Decisions to Make

1. **ID type:** `UmlId` as `uuid::Uuid`, `String`, or `u64`? Current uses `std::string` (hex). Prefer `Uuid` for global uniqueness, or `u64` for efficiency.

2. **Arena granularity:** One giant arena or type-specific arenas? Type-specific arenas (one slotmap per type) give better cache locality but more complex cross-references. Recommendation: single main arena + secondary maps for hot queries.

3. **Mutation pattern:** Direct struct mutation, or through `ModelArena` methods? Arena methods allow event emission and invariant enforcement. Prefer arena as sole mutation entry point.

4. **Null/undefined handling:** Current code uses `nullptr` for unset references. In Rust, use `Option<ObjectKey>` everywhere.

5. **Circular references:** Association ↔ Role is circular. In arena model, both can hold the other's key without issues (no borrow conflicts). `Rc/RefCell` should not be needed.

6. **Copy semantics:** Current `clone()` is broken for some types. In Rust, `Clone` should be implemented uniformly. Since all data is arena-resident, cloning a struct clones its keys (cheap), not the referenced objects (no deep-copy unless needed).

### 10.4 Potential Pitfalls

- **Enum merging:** The current `AssociationType` mixes structural and message types. Rust should split these into separate enums to avoid propagating the design mistake.
- **Qt thread model:** The model layer currently assumes single-threaded Qt event loop. Rust's `Send + Sync` on model types will need careful design if multiple threads observe model changes.
- **Undo/Redo:** The C++ model uses `QUndoCommand` for undoable operations (setNameCmd, etc.). Rust needs an equivalent command pattern. Consider a `ChangeLog` trait or a snapshot-based approach.
- **Instances auto-mirroring classifier attributes:** `UMLInstance` connects to classifier signals to auto-create `UMLInstanceAttribute` objects. This observer pattern must be ported carefully.
- **Reference counting (stereotypes):** `UMLStereotype` uses manual ref-counting. In Rust, this could use `Arc` or arena-based reference tracking.

### 10.5 File Organization

```
rust-rewrite/
├── model/
│   ├── src/
│   │   ├── lib.rs             — re-exports, ModelArena
│   │   ├── types.rs           — core enums (Visibility, ObjectType, etc.)
│   │   ├── element.rs         — UmlModelElement enum
│   │   ├── class.rs           — UmlClass, UmlInterface
│   │   ├── datatype.rs        — UmlDatatype
│   │   ├── enumeration.rs     — UmlEnumeration, UmlEnumLiteral
│   │   ├── entity.rs          — UmlEntity
│   │   ├── constraint.rs      — constraint types
│   │   ├── package.rs         — UmlPackage, UmlFolder
│   │   ├── component.rs       — UmlComponent, UmlArtifact
│   │   ├── actor.rs           — UmlActor, UmlUseCase
│   │   ├── node.rs            — UmlNode, UmlPort
│   │   ├── category.rs        — UmlCategory
│   │   ├── instance.rs        — UmlInstance, UmlInstanceAttribute
│   │   ├── association.rs     — UmlAssociation, UmlRole, AssociationType
│   │   ├── attribute.rs       — UmlAttribute, UmlEntityAttribute
│   │   ├── operation.rs       — UmlOperation, UmlParameter
│   │   ├── template.rs        — UmlTemplateParameter
│   │   ├── stereotype.rs      — UmlStereotype
│   │   ├── arena.rs           — ModelArena, ObjectKey
│   │   ├── event.rs           — ModelEventBus, ModelEvent
│   │   ├── xmi/
│   │   │   ├── mod.rs
│   │   │   ├── reader.rs      — XMI import
│   │   │   └── writer.rs      — XMI export
│   │   └── error.rs           — Error types
│   └── tests/
│       ├── roundtrip.rs       — read/write/diff tests
│       └── model_tests.rs     — unit tests
```

---

## Appendix A: Enum/Files Cross-Reference

| File | Class/Enum | Lines | Role |
|------|-----------|-------|------|
| `umlobject.h/cpp` | UMLObject | 337 + 1476 | Root class |
| `umlcanvasobject.h/cpp` | UMLCanvasObject | 106 + ~200 | Canvas items |
| `umlpackage.h/cpp` | UMLPackage | 82 + ~200 | Namespace |
| `umlfolder.h/cpp` | UMLFolder | 91 + ~300 | Top-level folder |
| `umlclassifier.h/cpp` | UMLClassifier | 182 + ~600 | Class/Interface |
| `umlenum.h/cpp` | UMLEnum | 69 + ~120 | Enumeration |
| `umldatatype.h/cpp` | UMLDatatype | 44 + ~120 | Primitive type |
| `umlentity.h/cpp` | UMLEntity | 103 + ~300 | DB entity |
| `umlcomponent.h/cpp` | UMLComponent | 43 + ~100 | Component |
| `umlartifact.h/cpp` | UMLArtifact | 63 + ~100 | Artifact |
| `umlactor.h/cpp` | UMLActor | 39 + ~60 | Actor |
| `umlusecase.h/cpp` | UMLUseCase | 35 + ~60 | Use case |
| `umlnode.h/cpp` | UMLNode | 41 + ~60 | Node |
| `umlport.h/cpp` | UMLPort | 41 + ~60 | Port |
| `umlcategory.h/cpp` | UMLCategory | 56 + ~80 | Generalization set |
| `umlinstance.h/cpp` | UMLInstance | 66 + ~170 | Object diagram instance |
| `umlclassifierlistitem.h/cpp` | UMLClassifierListItem | 54 + ~150 | Base for child items |
| `umlattribute.h/cpp` | UMLAttribute | 80 + ~380 | Attribute |
| `umlentityattribute.h/cpp` | UMLEntityAttribute | 81 + ~260 | DB column |
| `umloperation.h/cpp` | UMLOperation | 92 + ~700 | Operation |
| `umltemplate.h/cpp` | UMLTemplate | 53 + ~140 | Template parameter |
| `umlenumliteral.h/cpp` | UMLEnumLiteral | 59 + ~90 | Enum literal |
| `umlentityconstraint.h/cpp` | UMLEntityConstraint | 44 + ~40 | Constraint base |
| `umluniqueconstraint.h/cpp` | UMLUniqueConstraint | 73 + ~120 | UNIQUE constraint |
| `umlforeignkeyconstraint.h/cpp` | UMLForeignKeyConstraint | 116 + ~250 | FK constraint |
| `umlcheckconstraint.h/cpp` | UMLCheckConstraint | 67 + ~80 | CHECK constraint |
| `umlassociation.h/cpp` | UMLAssociation | 102 + ~600 | Relationship |
| `umlrole.h/cpp` | UMLRole | 62 + ~350 | Role/Endpoint |
| `umlstereotype.h/cpp` | UMLStereotype | 89 + ~200 | Stereotype |
| `umlinstanceattribute.h/cpp` | UMLInstanceAttribute | 60 + ~150 | Instance value |
| `basictypes.h` | Uml::* enums | 397 | All enums + ID type |
| `umlobjectprivate.h` | UMLObjectPrivate | 17 | D-pointer (isSaved) |
| 12 `*list.h` files | Various lists | ~15-35 each | List typedefs |

# Page Object Authoring Guide

This guide covers how to write UTAM page object definitions in JSON.
These definitions work with both the compiled path (`utam-compiler`)
and the runtime interpreter (`utam-runtime`).

For the full grammar specification, see [utam.dev/grammar/spec](https://utam.dev/grammar/spec).

## Minimal Page Object

The simplest possible page object — a root element with a CSS selector:

```json
{
  "root": true,
  "selector": { "css": ".my-component" }
}
```

This declares a page object that can be loaded from a page by finding
an element matching `.my-component`.

## Adding Elements

Elements are child components within the page object. Each element
has a name, a selector, and an optional type:

```json
{
  "root": true,
  "selector": { "css": ".login-form" },
  "elements": [
    {
      "name": "usernameInput",
      "type": ["editable"],
      "selector": { "css": "input[name='username']" }
    },
    {
      "name": "submitButton",
      "type": ["clickable"],
      "selector": { "css": "button[type='submit']" },
      "public": true
    }
  ]
}
```

### Element Types

| Type | What it enables |
|------|----------------|
| `"clickable"` | click, doubleClick, rightClick |
| `"editable"` | setText, clear, clearAndType, press |
| `"actionable"` | focus, blur, scrollIntoView, moveTo |
| `"draggable"` | dragAndDrop, dragAndDropByOffset |
| `"container"` | Generic container, no specific actions |
| `"frame"` | iframe handling with context switching |

Multiple types can be combined: `"type": ["clickable", "editable"]`

### Element Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `name` | string | required | Identifier used in methods |
| `type` | string/array | none | Action types or custom component reference |
| `selector` | object | none | How to find the element |
| `public` | bool | false | Whether a getter is generated |
| `nullable` | bool | false | Whether the element may not exist |
| `list` | bool | false | Whether the selector returns multiple elements |

## Selectors

### CSS Selectors (Most Common)

```json
{ "css": "button.submit" }
```

### Return All (Collections)

Find all matching elements instead of just the first:

```json
{
  "name": "todoItems",
  "type": ["clickable"],
  "selector": { "css": "li.todo-item", "returnAll": true },
  "list": true
}
```

### Parameterized Selectors

Use `%s` for string parameters and `%d` for numeric parameters:

```json
{
  "name": "tabByName",
  "selector": {
    "css": "button[data-tab='%s']",
    "args": [
      { "name": "tabName", "type": "string" }
    ]
  }
}
```

### Mobile Selectors

For mobile testing, alternative selector strategies:

```json
{ "accessid": "login-button" }
{ "classchain": "XCUIElementTypeButton[1]" }
{ "uiautomator": "new UiSelector().text(\"Submit\")" }
```

## Shadow DOM

Salesforce Lightning Web Components use shadow DOM extensively. Declare
shadow elements under a `shadow` key:

```json
{
  "root": true,
  "selector": { "css": "my-component" },
  "shadow": {
    "elements": [
      {
        "name": "innerButton",
        "type": ["clickable"],
        "selector": { "css": "button.action" }
      }
    ]
  }
}
```

The runtime automatically traverses the shadow root when resolving
these elements.

## Compose Methods

Compose methods define multi-step operations as sequences of
element actions:

```json
{
  "methods": [
    {
      "name": "login",
      "compose": [
        {
          "element": "usernameInput",
          "apply": "clearAndType",
          "args": [{ "name": "username", "type": "string" }]
        },
        {
          "element": "passwordInput",
          "apply": "clearAndType",
          "args": [{ "name": "password", "type": "string" }]
        },
        {
          "element": "submitButton",
          "apply": "click"
        }
      ]
    }
  ]
}
```

### Method Arguments

Arguments declared in compose steps become method parameters:

```json
{
  "name": "searchFor",
  "compose": [
    {
      "element": "searchInput",
      "apply": "clearAndType",
      "args": [{ "name": "query", "type": "string" }]
    }
  ]
}
```

This produces a method signature: `searchFor(query: string)`

Argument types: `"string"`, `"boolean"`, `"number"`

### Self-Referential Calls

A method can call another method on the same page object:

```json
{
  "name": "searchAndGetResults",
  "compose": [
    {
      "apply": "performSearch",
      "args": [{ "name": "query", "type": "string" }]
    },
    {
      "element": "resultsList",
      "returnElement": true
    }
  ]
}
```

### Returning Elements

Use `"returnElement": true` to return an element reference:

```json
{
  "element": "resultsList",
  "returnElement": true
}
```

### Chaining

Chain from the previous step's result:

```json
{
  "element": "parentComponent",
  "apply": "getChild"
},
{
  "chain": true,
  "apply": "getText"
}
```

## Filters and Matchers

Filter a list of elements using matchers:

```json
{
  "element": "todoItems",
  "filter": [
    {
      "matcher": { "type": "isVisible" }
    }
  ]
}
```

### Matcher Types

| Matcher | Description |
|---------|-------------|
| `isVisible` | Element is displayed |
| `isEnabled` | Element is enabled |
| `isPresent` | Element is in the DOM |
| `stringContains` | Text contains a substring |
| `stringEquals` | Text equals a value |
| `startsWith` | Text starts with a prefix |
| `endsWith` | Text ends with a suffix |

## Descriptions

Simple description:

```json
{ "description": "Login form component" }
```

Rich description with author:

```json
{
  "description": {
    "text": [
      "Represents the login page.",
      "Access username, password, or submit button."
    ],
    "author": "Salesforce"
  }
}
```

## beforeLoad

Wait for conditions before the page object is considered loaded:

```json
{
  "beforeLoad": [
    {
      "apply": "waitFor",
      "args": [
        {
          "type": "function",
          "predicate": [
            {
              "element": "root",
              "apply": "isPresent"
            }
          ]
        }
      ]
    }
  ]
}
```

## Custom Component Types

Reference another page object as an element type:

```json
{
  "name": "profile",
  "type": "utam-global/pageObjects/userProfileCardTrigger",
  "selector": { "css": ".userProfileCardTriggerRoot" },
  "public": true
}
```

This tells the framework that the `profile` element is itself a page
object defined elsewhere, enabling compositional page object patterns.

## Complete Example

Here is the Salesforce login page object — a real-world example showing
elements, compose methods, and cross-method calls:

```json
{
  "description": {
    "text": ["Represents the login page."],
    "author": "Salesforce"
  },
  "selector": { "css": "div#content" },
  "root": true,
  "elements": [
    {
      "name": "userName",
      "public": true,
      "type": ["editable"],
      "selector": { "css": "input[type='email']" }
    },
    {
      "name": "password",
      "public": true,
      "type": ["editable"],
      "selector": { "css": "input[type='password']" }
    },
    {
      "name": "submitBtn",
      "public": true,
      "type": ["clickable"],
      "selector": { "css": "input[type='submit']" }
    }
  ],
  "methods": [
    {
      "name": "login",
      "description": "Enter credentials and submit",
      "compose": [
        { "element": "userName", "apply": "waitForVisible" },
        { "element": "userName", "apply": "setText",
          "args": [{ "name": "userNameStr", "type": "string" }] },
        { "element": "password", "apply": "waitForVisible" },
        { "element": "password", "apply": "setText",
          "args": [{ "name": "passwordStr", "type": "string" }] },
        { "element": "submitBtn", "apply": "waitForVisible" },
        { "element": "submitBtn", "apply": "click" }
      ]
    }
  ]
}
```

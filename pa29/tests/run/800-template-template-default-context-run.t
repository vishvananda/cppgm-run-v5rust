// N3485 14.1p10, 14.3.3: a template template-parameter defaulted to an alias
// template declared before the class (libc++'s tuple and _And) names that
// alias template wherever the constructor template is used, so
// __is_constructible sees the constructor only when its enable_if holds

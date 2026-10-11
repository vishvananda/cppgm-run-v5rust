// VALIDATION: compile-pass
// An imported function template is visible from its using-declaration point;
// token offsets from different source files are not comparable.

#include "300-imported-function-template-default-noexcept.h"

template<class T> T&& declval() noexcept;

namespace detector
{
using origin::swap;

template<class T, bool Value = noexcept(swap(declval<T&>(), declval<T&>()))>
struct trait { static constexpr bool value = Value; };
}

static_assert(detector::trait<int>::value,
              "the imported function template must be visible");

int main() { return 0; }

// VALIDATION: compile-pass
// Boost.Contract reduction: two captureless closure arguments parsed from a
// non-primary function body can retain the same collapsed body-token anchor.
// Their distinct typed lambda expressions must still receive distinct closure
// class identities while chained member templates are deduced.

#include "300-nonprimary-chained-member-template-lambda-anchor-collision.h"

int nonprimary_chained_lambda::side_effect;

int main()
{
  return nonprimary_chained_lambda::run() == 0 &&
         nonprimary_chained_lambda::side_effect == 2 ? 0 : 1;
}

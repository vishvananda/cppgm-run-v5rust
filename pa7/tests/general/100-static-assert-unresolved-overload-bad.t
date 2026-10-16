// A contextual bool target cannot resolve an overloaded function designation.
int function(int);
int function(double);
static_assert(function, "unresolved overload");

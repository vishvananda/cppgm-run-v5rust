// PA29 GNU typeof still needs a resolved operand type.
int function(int);
int function(long);
typedef __typeof__(function) result;

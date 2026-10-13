int function(int);
int take(...);
int (*indirect)(...);
int call() { return take(function) + take(&function) + indirect(function); }

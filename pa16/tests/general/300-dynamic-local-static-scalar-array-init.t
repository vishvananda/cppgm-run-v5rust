// VALIDATION: compile-pass
int next();
int read() { static int values[1] = {next()}; return values[0]; }

typedef const int input[4];

int array_parameter(int values[4], int (&bytes)[sizeof(values)]);
int alias_parameter(input values, int (&bytes)[sizeof(values)]);
int function_parameter(int callback(int), int (&bytes)[sizeof(callback)]);
int qualified_parameter(const int value, int (&bytes)[sizeof(value)]);

int main() { return 0; }

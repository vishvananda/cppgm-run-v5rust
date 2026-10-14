// expect exit 0
int calls;
int &identity(int &value) { ++calls; return value; }
int take(const double &value) { return value == 3.0 ? 0 : 1; }
int main() {
    int value = 1;
    int &alias{value};
    identity(alias) += 2;
    const int &temporary = true ? value + 1 : value + 2;
    const float converted = 3.0f;
    volatile int &&volatile_temporary = 5;
    int result = take(converted);
    int &cond__1 = alias;
    int refarg__3 = 4;
    const int &other = temporary;
    int selected_value = result ? alias : value;
    bool success = !result;
    (void)identity(cond__1);
    return result || calls != 2 || value != 3 || other != refarg__3 || volatile_temporary != 5 || selected_value != 3 || !success;
}

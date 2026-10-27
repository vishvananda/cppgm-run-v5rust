struct Value {
    Value(int);
private:
    Value(Value &&);
};
int main() {
    Value values[2] = {1, 2};
    return 0;
}

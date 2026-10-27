int main() {
    char direct[4]("abc");
    char nested[4](("xyz"));
    return direct[1] + nested[2] - 'b' - 'z';
}

enum class kind { value };

void emit(kind, int);

void call() {
  emit(kind::value, 1);
}

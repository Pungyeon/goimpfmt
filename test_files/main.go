package main

import (
	"fmt"
	"os"

	json "github.com/Pungyeon/required/pkg/json"

	"github.com/Pungyeon/test_files/pkg/person"
)

func main() {
	p := person.Person{
		Name: "Lasse",
	}

	data, err := json.Marshal(p)
	if err != nil {
		panic(err)
	}
	fmt.Println(os.Args)
	fmt.Println(string(data))
}

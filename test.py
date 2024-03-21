"""
test = {}
def a(id: str, attr: str, v: int):
    b = test.get(id)
    if b is None:
        b = {"f": 1}

    b[attr] = v

    test[id] = b


a("d", "f", 2)
"""


class Hello:
    test: str

    def __init__(self, test: str) -> None:
        test = ""
        pass


a = Hello("HEe")

getattr()

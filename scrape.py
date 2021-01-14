import requests
import re
from bs4 import BeautifulSoup

URL = "https://web6.karlsruhe.de/service/abfall/akal/akal.php"
DATE = re.compile(r"\d\d\.\d\d\.2021")


class StreetQuery:
    def __init__(self, tag):
        self.street = tag.string.strip()
        self.from_letter = self.street[0]
        self.value = int(tag["value"])

        if self.from_letter == "Z":
            self.to_letter = "["
        else:
            self.to_letter = chr(ord(self.from_letter) + 1)

    @property
    def params(self):
        return dict(von=self.from_letter, bis=self.to_letter)

    @property
    def data(self):
        return dict(anzeigen="anzeigen", strasse=self.value, hausnr="")


def get_street_queries():
    page = requests.post(URL, params=dict(von="A", bis="[")).text
    soup = BeautifulSoup(page, features='lxml')
    return [StreetQuery(tag) for tag in soup.body.find("select")]


def get_date(query):
    page = requests.post(URL, params=query.params, data=query.data).text
    soup = BeautifulSoup(page, features="lxml")
    rows = soup.body.find("div", attrs={"id": "foo"}).table.find_all("tr")
    td = rows[-1].find_all("td")[-2]
    match = DATE.search(td.text)

    if match:
        return match.group(0)

    return None


for query in get_street_queries():
    date = get_date(query)

    if date:
        print(f"{query.street};{date}")

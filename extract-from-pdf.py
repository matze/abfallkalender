import csv
import re
import datetime
import pdfplumber

ENTRY = re.compile(r"([A-Za-z][\w\d -]+) (Mo|Di|Mi|Do|Fr|Sa|So) (\d\d)\.(\d\d)\.23")

with pdfplumber.open("22045_AfA_Sperrmuellseiten_2023_RZ_Druck.pdf") as pdf:
    entries = []

    for page in pdf.pages:
        text = page.extract_text()

        for match in ENTRY.finditer(text):
            street = match.group(1).strip()
            garbage = street.find("  ")

            if garbage > 0:
                street = street[garbage + 2:]

            day = int(match.group(3))
            month = int(match.group(4))
            date = datetime.date(2023, month, day)
            entries.append((street, date))

        entries.sort(key=lambda e: e[1])

    with open("sperrmüll.csv", "w", newline="") as csvfile:
        writer = csv.writer(csvfile)

        for entry in entries:
            writer.writerow(entry)

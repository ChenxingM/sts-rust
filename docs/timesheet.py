import os
import re
import pathlib
import json
import types
import traceback
from typing import Any, NamedTuple


class Frame(NamedTuple):
    frame: int
    cell: int


class Layer(NamedTuple):
    name: str
    frames: list[Frame]


class Timesheet:
    def __init__(self, path: pathlib.Path, name: str, frame_count: int, layers: list[Layer]) -> None:
        self.path = path
        self.name = name
        self.frame_count = frame_count
        self.layers = layers

        for layer in layers:
            frames = layer.frames

            if frames[0].frame != 0:
                frames.insert(0, Frame(0, 0))

            for i in range(len(frames) - 1, 0, -1):
                if frames[i].cell == frames[i - 1].cell:
                    frames.pop(i)


class TDTS(Timesheet):
    def __init__(self, path: pathlib.Path, timeSheet_name: str, timeTable: dict[str, Any]) -> None:
        name = f"{path.name}->{timeSheet_name}->{timeTable['name']}"
        layers, frame_count = self.__read(timeTable)
        super().__init__(path, name, frame_count, layers)

        self.raw_table = timeTable

    def __read(self, timeTable: dict[str, Any]) -> tuple[list[Layer], int]:
        frame_count: int = timeTable["duration"]

        for field in timeTable["fields"]:
            if field["fieldId"] == 4:
                tracks = field["tracks"]
                break
        else:
            tracks = None

        for timeTableHeader in timeTable["timeTableHeaders"]:
            if timeTableHeader["fieldId"] == 4:
                names = timeTableHeader["names"]
                break
        else:
            names = None

        layers: list[Layer] = []
        if tracks is not None and names is not None:
            for track in tracks:
                trackNo = track["trackNo"]
                name = names[trackNo]
                frames: list[Frame] = []
                for frame in track["frames"]:
                    value = frame["data"][0]["values"][0]
                    if value == "SYMBOL_NULL_CELL":
                        cell = 0
                    else:
                        try:
                            cell = int(value)
                        except:
                            traceback.print_exc()
                            continue
                    frames.append(Frame(frame["frame"], cell))
                layers.append(Layer(name, frames))

        return layers, frame_count


class XDTS(Timesheet):
    def __init__(self, path: pathlib.Path, timeTable: dict[str, Any]) -> None:
        name = f"{path.name}->{timeTable['name']}"
        layers, frame_count = self.__read(timeTable)
        super().__init__(path, name, frame_count, layers)

    def __read(self, timeTable: dict[str, Any]) -> tuple[list[Layer], int]:
        frame_count = timeTable["duration"]

        field = timeTable["fields"][0]
        fieldId = field["fieldId"]
        tracks = field["tracks"]

        for timeTableHeader in timeTable["timeTableHeaders"]:
            if timeTableHeader["fieldId"] == fieldId:
                names = timeTableHeader["names"]
                break
        else:
            names = None

        re_num = re.compile(r"\d+$")
        layers: list[Layer] = []
        if names is not None:
            for track in tracks:
                trackNo = track["trackNo"]
                name = names[trackNo]
                frames: list[Frame] = []
                for frame in track["frames"]:
                    value = frame["data"][0]["values"][0]
                    if value == "SYMBOL_NULL_CELL":
                        cell = 0
                    elif value in ["SYMBOL_TICK_1", "SYMBOL_TICK_2", "SYMBOL_HYPHEN"]:
                        continue
                    else:
                        match = re_num.search(value)
                        if match is None:
                            try:
                                raise Exception("NotNumberError:" + value)
                            except:
                                traceback.print_exc()
                        else:
                            cell = int(match.group())
                    frames.append(Frame(frame["frame"], cell))
                layers.append(Layer(name, frames))

        return layers, frame_count


class STS(Timesheet):
    def __init__(self, path: pathlib.Path) -> None:
        with open(path, "rb") as f:
            f.int = types.MethodType(lambda self, length: int.from_bytes(self.read(length), byteorder='little', signed=False), f)  # type:ignore
            f.read(1)
            if f.read(17).decode() != "ShiraheiTimeSheet":
                raise Exception("NotSTSFileFormatError")

            layer_count = f.int(1)  # type:ignore
            frame_count = f.int(2)  # type:ignore

            f.read(2)

            frames_list: list[list[Frame]] = []
            for i in range(layer_count):
                frames: list[Frame] = []
                for j in range(frame_count):
                    cell = f.int(2)  # type:ignore
                    if len(frames) == 0 or frames[-1].cell != cell:
                        frames.append(Frame(j, cell))
                frames_list.append(frames)

            layers: list[Layer] = []
            for i in range(layer_count):
                name_length = f.int(1)  # type:ignore
                name_bytes = f.read(name_length)
                name = name_bytes.decode("shift-jis")
                layers.append(Layer(name, frames_list[i]))

            name = os.path.basename(path)
            super().__init__(path, name, frame_count, layers)


def load(file_path: pathlib.Path) -> list[Timesheet]:
    ext = file_path.suffix.lower()

    if ext == ".sts":
        return [STS(file_path)]
    elif ext == ".tdts":
        with open(file_path, encoding="utf-8") as f:
            d = json.loads("\n".join(f.readlines()[1:]))
        dst: list[Timesheet] = []
        for timeSheet in d["timeSheets"]:
            for timeTable in timeSheet["timeTables"]:
                if "fields" in timeTable:
                    tdts = TDTS(file_path, timeSheet["header"]["cut"], timeTable)
                    dst.append(tdts)
        return dst
    elif ext == ".xdts":
        with open(file_path, encoding="utf-8") as f:
            d = json.loads("\n".join(f.readlines()[1:]))
        return [XDTS(file_path, timeTable) for timeTable in d["timeTables"]]
    else:
        raise Exception(f"TimeseetFileTypeError:{file_path}")

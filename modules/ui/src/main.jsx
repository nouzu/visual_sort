import React from "react";
import ReactDOM from "react-dom/client";

import "@fontsource/roboto/300.css";
import "@fontsource/roboto/400.css";
import "@fontsource/roboto/500.css";
import "@fontsource/roboto/700.css";

import "./global.css"

import CssBaseline from "@mui/material/CssBaseline";
import Box from "@mui/material/Box";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemText from "@mui/material/ListItemText";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListSubheader from "@mui/material/ListSubheader";
import Divider from "@mui/material/Divider";
import Stack from "@mui/material/Stack";
import Slider from "@mui/material/Slider"

import ShuffleIcon from '@mui/icons-material/Shuffle';
import BubbleChartIcon from '@mui/icons-material/BubbleChart';
import SpeedIcon from '@mui/icons-material/Speed';
import SwitchAccessShortcutAddIcon from '@mui/icons-material/SwitchAccessShortcutAdd';
import CallMergeIcon from '@mui/icons-material/CallMerge';
import SettingsBackupRestoreIcon from '@mui/icons-material/SettingsBackupRestore';

import init, {shuffle, reverse, bubble_sort, insertion_sort, merge_sort} from "renderer"

const App = () => {
  const actions = [
    {
      title: "Shuffle",
      icon: <ShuffleIcon/>,
      onClick: () => shuffle()
    },
    {
      title: "Reverse",
      icon: <SettingsBackupRestoreIcon/>,
      onClick: () => reverse()
    }
  ];

  const algorithms = [
    {
      title: "Bubble Sort",
      icon: <BubbleChartIcon/>,
      onClick: () => runAlgorithm(bubble_sort),
    },
    {
      title: "Insertion Sort",
      icon: <SwitchAccessShortcutAddIcon/>,
      onClick: () => runAlgorithm(insertion_sort)
    },
    {
      title: "Merge Sort",
      icon: <CallMergeIcon/>,
      onClick: () => runAlgorithm(merge_sort)
    }
  ]

  const runAlgorithm = (func) => {
    setRunning(true);

    func(speed)
      .then(() => setRunning(false));
  }

  const [initializing, setInitializing] = React.useState(true);
  const [running, setRunning] = React.useState(false);
  const [speed, setSpeed] = React.useState(256);

  React.useEffect(() => {
    if (initializing) {
      init().then(() => setInitializing(false));
    }
  })

  const disabled = running || initializing;

  return (
    <React.StrictMode>
      <CssBaseline/>
      <Box sx={{display: "flex"}}>
        <Box component="main" id="m" sx={{width: 800, height: 600}}>
          {/* the canvas will be here */}
        </Box>
        <Box component="aside" sx={{width: 300, height: 600}}>
          <List subheader={<ListSubheader>Actions</ListSubheader>}>
            {actions.map((action) => (
              <ListItem key={action.title} sx={{p: 0}}>
                <ListItemButton disabled={disabled} onClick={action.onClick}>
                  <ListItemIcon>
                    {action.icon}
                  </ListItemIcon>
                  <ListItemText>
                    {action.title}
                  </ListItemText>
                </ListItemButton>
              </ListItem>
            ))}
          </List>
          <Divider/>
          <List>
            {algorithms.map((algorithm) => (
              <ListItem key={algorithm.title} sx={{p: 0}}>
                <ListItemButton disabled={disabled} onClick={algorithm.onClick}>
                  <ListItemIcon>
                    {algorithm.icon}
                  </ListItemIcon>
                  <ListItemText>
                    {algorithm.title}
                  </ListItemText>
                </ListItemButton>
              </ListItem>
            ))}
          </List>
          <Divider/>
          <Box sx={{m: 2}}>
            <Stack
              spacing={2}
              direction="row"
              alignItems="center"
              sx={{px: 2}}
            >
              <SpeedIcon sx={{mr: 1}}/>
              <Slider
                disabled={disabled}
                min={1}
                max={512}
                defaultValue={256}
                value={speed}
                onChange={(_, n) => setSpeed(n)}
                valueLabelDisplay="auto"
              />
            </Stack>
          </Box>
        </Box>
      </Box>
    </React.StrictMode>
  )
}

const root = document.getElementById("root")

ReactDOM.createRoot(root)
  .render(<App/>);
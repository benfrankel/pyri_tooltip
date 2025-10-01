# Version 0.6.0

- Updated to Bevy 0.17

# Version 0.5.0

- Improved transform propagation system ordering
- Added "follow cursor" placement

# Version 0.4.3

- Added `line_height` and `font_smoothing` fields to `RichText` component

# Version 0.4.2

- Fixed tooltip not updating while targetting the same entity

# Version 0.4.1

- Fixed tooltip placement for non-unit DPI

# Version 0.4.0

- Renamed `PrimaryTooltip` -> `TooltipSettings` resource
- Added `TooltipSettings::enabled` flag

# Version 0.3.2

- Fixed position rounding edge cases

# Version 0.3.1

- Improved rounding for odd-width tooltip positions

# Version 0.3.0

- **Updated to Bevy 0.16**
- **Added `no_std` support**
- Renamed `TooltipSet` -> `TooltipSystems` system set

# Version 0.2.1

- Added `RichTextSystems` system set
- Fixed some system ordering issues
- Fixed errors with `bevy_reflect` feature disabled

# Version 0.2.0

- **Updated to Bevy 0.15**
- Added `RichText` component
- Added `TextSection` type
- Added `TextStyle` type

# Version 0.1.0

- **Initial release**
- Added `TooltipPlugin` plugin
- Added `PrimaryTooltip` resource
- Added `TooltipSet` system set
- Added `Tooltip` component
    - Added `TooltipActivation` field
    - Added `TooltipDismissal` field
    - Added `TooltipTransfer` field
    - Added `TooltipPlacement` field
    - Added `TooltipContent` field
- Added `bevy_reflect` feature